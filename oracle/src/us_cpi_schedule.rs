use std::fs;

use anyhow::{anyhow, Result};
use chrono::{DateTime, Utc};
use scraper::{Html, Selector};

use crate::utils::parse_date;

pub async fn load_cpi_schedule() -> Result<Vec<CpiSchedule>> {
	// let client = Client::new();
	// let page = client
	// 	.get("https://www.bls.gov/schedule/news_release/cpi.htm")
	// 	.send()
	// 	.await?
	// 	.text()
	// 	.await?;
	// println!("{:?}", page);
	let schedule = fs::read("cpi_schedule.html")?;
	let page = String::from_utf8(schedule)?;
	parse_schedule(page)
}

fn parse_schedule(html: String) -> Result<Vec<CpiSchedule>> {
	let mut cpi_schedule = vec![];
	let document = Html::parse_document(&html);
	let selector =
		Selector::parse("#bodytext .release-list tbody tr").map_err(|e| anyhow!(e.to_string()))?;
	let td_selector = Selector::parse("td").map_err(|e| anyhow!(e.to_string()))?;
	for element in document.select(&selector) {
		let mut tds = element.select(&td_selector);
		let ref_month = tds
			.next()
			.ok_or_else(|| anyhow!("Failed to get ref month"))?
			.text()
			.collect::<String>();
		let release_date = tds
			.next()
			.ok_or_else(|| anyhow!("Failed to get release date"))?
			.text()
			.collect::<String>();

		let release_date = parse_date(&release_date, vec!["%b. %d, %Y", "%b %d, %Y"])?; // Nov. 14, 2023

		// ref month looks like December 2023
		let ref_month = parse_date(&format!("1 {}", ref_month), vec!["%d %B %Y"])?;

		cpi_schedule.push(CpiSchedule { ref_month, release_date });
	}

	Ok(cpi_schedule)
}

#[derive(Debug, Clone)]
pub struct CpiSchedule {
	pub ref_month: DateTime<Utc>,
	pub release_date: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[tokio::test]
	async fn test_parse_schedule() {
		let html = r#"
		<!DOCTYPE HTML>
		<html lang="en-us"><body>
		<div id="main-content-td" class="main-content">
			<div id="bodytext" class="verdana md">

				<h2>Schedule of Releases for the Consumer Price Index</h2>
				<hr>
				<table class="release-list">
				<thead>
				<tr>
				<th>Reference Month</th>
				<th>Release Date</th>
				<th>Release Time</th>
				</tr>
				</thead>
				<tbody>
				<tr class="release-list-even-row">
				<td>October 2023</td>
				<td>Nov. 14, 2023</td>
				<td>08:30 AM</td>
				</tr>
				<tr class="release-list-odd-row">
				<td>November 2023</td>
				<td>Dec. 12, 2023</td>
				<td>08:30 AM</td>
				</tr>
				<tr class="release-list-even-row">
				<td>December 2023</td>
				<td>Jan. 11, 2024</td>
				<td>08:30 AM</td>
				</tr>
				<tr class="release-list-odd-row">
				<td>January 2024</td>
				<td>Feb. 13, 2024</td>
				<td>08:30 AM</td>
				</tr>
				<tr class="release-list-even-row">
				<td>February 2024</td>
				<td>Mar. 12, 2024</td>
				<td>08:30 AM</td>
				</tr>
				<tr class="release-list-odd-row">
				<td>March 2024</td>
				<td>Apr. 10, 2024</td>
				<td>08:30 AM</td>
				</tr>
				<tr class="release-list-even-row">
				<td>April 2024</td>
				<td>May 15, 2024</td>
				<td>08:30 AM</td>
				</tr>
				<tr class="release-list-odd-row">
				<td>May 2024</td>
				<td>Jun. 12, 2024</td>
				<td>08:30 AM</td>
				</tr>
				<tr class="release-list-even-row">
				<td>June 2024</td>
				<td>Jul. 11, 2024</td>
				<td>08:30 AM</td>
				</tr>
				<tr class="release-list-odd-row">
				<td>July 2024</td>
				<td>Aug. 14, 2024</td>
				<td>08:30 AM</td>
				</tr>
				<tr class="release-list-even-row">
				<td>August 2024</td>
				<td>Sep. 11, 2024</td>
				<td>08:30 AM</td>
				</tr>
				<tr class="release-list-odd-row">
				<td>September 2024</td>
				<td>Oct. 10, 2024</td>
				<td>08:30 AM</td>
				</tr>
				<tr class="release-list-even-row">
				<td>October 2024</td>
				<td>Nov. 13, 2024</td>
				<td>08:30 AM</td>
				</tr>
				<tr class="release-list-odd-row">
				<td>November 2024</td>
				<td>Dec. 11, 2024</td>
				<td>08:30 AM</td>
				</tr>
				</tbody>
				</table>
			</div>
		</div>
	</body></html>"#;
		let schedule = parse_schedule(html.to_string()).unwrap();
		assert!(schedule.len().ge(&12));
		assert_eq!(schedule[0].ref_month.to_string(), "2023-10-01 00:00:00 UTC");
		assert_eq!(schedule[0].release_date.to_string(), "2023-11-14 00:00:00 UTC");
		assert_eq!(schedule[1].ref_month.to_string(), "2023-11-01 00:00:00 UTC");
		assert_eq!(schedule[1].release_date.to_string(), "2023-12-12 00:00:00 UTC");
	}

	#[tokio::test]
	async fn test_can_get_schedule() {
		let schedule = load_cpi_schedule().await.unwrap();
		assert!(schedule.len().ge(&12));
	}
}
