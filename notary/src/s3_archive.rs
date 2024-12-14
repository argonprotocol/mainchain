use crate::server::ArchiveSettings;
use anyhow::bail;
use argon_notary_apis::{get_header_bucket, get_notebook_bucket, Error, Error::ArchiveError};
use argon_primitives::{NotaryId, NotebookNumber};
use base64::Engine;
use md5::{Digest, Md5};
use rusoto_core::{request::BufferedHttpResponse, Region};
use rusoto_credential::DefaultCredentialsProvider;
use rusoto_s3::{
	DeleteBucketRequest, HeadBucketError, HeadBucketRequest, PutBucketPolicyRequest,
	PutObjectOutput, PutObjectRequest, S3Client, S3,
};
use std::{env, str::FromStr};
use tokio::task;
use tracing::{info, trace, warn};
use uuid::Uuid;

#[derive(Clone)]
pub struct S3Archive {
	pub bucket_name: String,
	pub credentials_provider: DefaultCredentialsProvider,
	pub notary_id: NotaryId,
	pub client: S3Client,
	remove_bucket_on_drop: bool,
}

impl Drop for S3Archive {
	fn drop(&mut self) {
		if self.remove_bucket_on_drop {
			let client = self.client.clone(); // Clone the client to use in async context
			let bucket_name = self.bucket_name.clone();

			task::spawn(async move {
				match client
					.delete_bucket(DeleteBucketRequest {
						bucket: bucket_name.clone(),
						expected_bucket_owner: None,
					})
					.await
				{
					Ok(_) => info!("Deleted bucket {}", bucket_name),
					Err(err) => warn!("Failed to delete bucket: {:?}", err),
				}
			});
			info!("Deleted bucket {}", self.bucket_name);
		}
	}
}

impl S3Archive {
	/// Get the region from the region string and endpoint string
	/// AWS, Digital Ocean, Yandex and Wasabi regions will all automatically translate
	pub fn get_region(region: String, endpoint: Option<String>) -> anyhow::Result<Region, Error> {
		Ok(match endpoint {
			Some(endpoint) => Region::Custom { name: region, endpoint: endpoint.to_string() },
			None => Region::from_str(&region)
				.map_err(|e| ArchiveError(format!("Error parsing region {e}")))?,
		})
	}

	/// Creates a public access bucket for testing. NOTE: you need to have minio running on
	/// localhost:9000
	pub async fn rand_minio_test_bucket(
		notary_id: NotaryId,
		archive_bucket: Option<String>,
	) -> anyhow::Result<(S3Archive, ArchiveSettings)> {
		let access_key = "minioadmin";
		let secret_key = "minioadmin";
		env::set_var("AWS_ACCESS_KEY_ID", access_key);
		env::set_var("AWS_SECRET_ACCESS_KEY", secret_key);
		let minio_endpoint = "http://localhost:9000".to_string();

		let bucket_name = archive_bucket.unwrap_or(format!("notary-archives-{}", Uuid::new_v4()));
		let credentials_provider = DefaultCredentialsProvider::new()?;
		let region = Self::get_region("us-east-1".to_string(), Some(minio_endpoint.clone()))?;
		let client = S3Client::new_with(
			rusoto_core::request::HttpClient::new()?,
			credentials_provider.clone(),
			region,
		);
		if let Err(e) = client
			.head_bucket(HeadBucketRequest {
				bucket: bucket_name.clone(),
				expected_bucket_owner: None,
			})
			.await
		{
			if matches!(&e, rusoto_core::RusotoError::Service(HeadBucketError::NoSuchBucket(_))) ||
				matches!(&e, rusoto_core::RusotoError::Unknown(BufferedHttpResponse { headers, .. }) if headers.get("x-minio-error-code").cloned().unwrap_or_default() == "NoSuchBucket")
			{
				let _ = client
					.create_bucket(rusoto_s3::CreateBucketRequest {
						acl: Some("public-read".to_string()),
						bucket: bucket_name.clone(),
						..Default::default()
					})
					.await
					.inspect_err(|e| {
						warn!("Failed to create bucket  {}: {:?}", bucket_name.clone(), e);
					})?;

				client
					.put_bucket_policy(PutBucketPolicyRequest {
						bucket: bucket_name.clone(),
						policy: format!(
							r#"{{
					"Version": "2012-10-17",
					"Statement": [
						{{
							"Sid": "PublicReadGetObject",
							"Effect": "Allow",
							"Principal": "*",
							"Action": "s3:GetObject",
							"Resource": "arn:aws:s3:::{}/**"
						}}
					]
				}}"#,
							bucket_name.clone()
						),
						..Default::default()
					})
					.await
					.inspect_err(|e| {
						warn!("Failed to put bucket policy {}: {:?}", bucket_name.clone(), e);
					})?;
			} else {
				bail!("Error checking for bucket in minio: {:?}", e.to_string());
			}
		}

		let buckets = Self {
			bucket_name: bucket_name.clone(),
			notary_id,
			client,
			credentials_provider,
			remove_bucket_on_drop: true,
		};

		Ok((
			buckets,
			ArchiveSettings { archive_host: format!("{}/{}", minio_endpoint, bucket_name) },
		))
	}

	pub async fn new(
		notary_id: NotaryId,
		region: Region,
		bucket_name: String,
	) -> anyhow::Result<S3Archive, Error> {
		let credentials_provider = DefaultCredentialsProvider::new()
			.map_err(|e| ArchiveError(format!("Error creating credentials {e}")))?;

		let client = S3Client::new_with(
			rusoto_core::request::HttpClient::new().map_err(|e| ArchiveError(e.to_string()))?,
			credentials_provider.clone(),
			region,
		);
		client
			.head_bucket(HeadBucketRequest {
				bucket: bucket_name.clone(),
				expected_bucket_owner: None,
			})
			.await
			.map_err(|e| ArchiveError(e.to_string()))?;

		Ok(Self {
			bucket_name,
			notary_id,
			client,
			credentials_provider,
			remove_bucket_on_drop: false,
		})
	}

	async fn put_public(
		&self,
		key: String,
		body: Vec<u8>,
	) -> anyhow::Result<PutObjectOutput, Error> {
		let content_length = body.len() as i64;
		let digest = Md5::digest(body.as_slice());
		let digest = base64::engine::general_purpose::STANDARD.encode(digest);
		let response_data = self
			.client
			.put_object(PutObjectRequest {
				key,
				bucket: self.bucket_name.clone(),
				acl: Some("public-read".to_string()),
				body: Some(body.into()),
				content_type: Some("application/octet-stream".to_string()),
				content_length: Some(content_length),
				content_md5: Some(digest),
				..Default::default()
			})
			.await
			.map_err(|e| ArchiveError(e.to_string()))?;
		Ok(response_data)
	}

	pub async fn put_notebook(
		&self,
		notebook_number: NotebookNumber,
		notebook: Vec<u8>,
	) -> anyhow::Result<(), Error> {
		let bucket_path = get_notebook_bucket(self.notary_id);
		let key = format!("{}/{}.scale", bucket_path, notebook_number);
		let res = self.put_public(key, notebook).await?;
		trace!(?res, notebook_number, "Put header");
		Ok(())
	}

	pub async fn put_header(
		&self,
		notebook_number: NotebookNumber,
		header: Vec<u8>,
	) -> anyhow::Result<(), Error> {
		let bucket_path = get_header_bucket(self.notary_id);
		let key = format!("{}/{}.scale", bucket_path, notebook_number);
		let res = self.put_public(key, header).await?;
		trace!(?res, notebook_number, "Put header");
		Ok(())
	}
}
