
/// Allows for calling $method with appropriate crypto impl.
#[macro_export]
macro_rules! with_crypto_scheme {
	(
		$scheme:expr,
		$method:ident ( $($params:expr),* $(,)?) $(,)?
	) => {
		$crate::with_crypto_scheme!($scheme, $method<>($($params),*))
	};
	(
		$scheme:expr,
		$method:ident<$($generics:ty),*>( $( $params:expr ),* $(,)?) $(,)?
	) => {
		match $scheme {
			$crate::CryptoScheme::Ecdsa => {
				$method::<sp_core::ecdsa::Pair, $($generics),*>($($params),*)
			}
			$crate::CryptoScheme::Sr25519 => {
				$method::<sp_core::sr25519::Pair, $($generics),*>($($params),*)
			}
			$crate::CryptoScheme::Ed25519 => {
				$method::<sp_core::ed25519::Pair, $($generics),*>($($params),*)
			}
		}
	};
}