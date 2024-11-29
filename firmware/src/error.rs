#[derive(Debug, Copy, Clone)]
pub enum Infallible {}

impl core::fmt::Display for Infallible {
    fn fmt(&self, _: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {}
    }
}

impl core::error::Error for Infallible {}

impl From<core::convert::Infallible> for Infallible {
    fn from(err: core::convert::Infallible) -> Self {
        match err {}
    }
}

impl<E1, E2> From<embedded_hal_bus::spi::DeviceError<E1, E2>> for Infallible
where
    Infallible: From<E1> + From<E2>,
{
    fn from(err: embedded_hal_bus::spi::DeviceError<E1, E2>) -> Self {
        match err {
            embedded_hal_bus::spi::DeviceError::Spi(spi) => Self::from(spi),
            embedded_hal_bus::spi::DeviceError::Cs(cs) => Self::from(cs),
        }
    }
}
