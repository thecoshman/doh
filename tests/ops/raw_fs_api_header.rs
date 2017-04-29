use reqwest::header::{HeaderFormat, Header};
use doh::ops::RawFsApiHeader;
use reqwest::HyperError;
use std::fmt;


#[test]
fn header_name() {
    assert_eq!(RawFsApiHeader::header_name(), "X-Raw-Filesystem-API");
}

#[test]
fn parse_header_correct() {
    assert_eq!(RawFsApiHeader::parse_header(&[vec![b'1']]).unwrap(), RawFsApiHeader(true));
    assert_eq!(RawFsApiHeader::parse_header(&[vec![b'0']]).unwrap(), RawFsApiHeader(false));
}

#[test]
fn parse_header_incorrect() {
    assert_eq!(RawFsApiHeader::parse_header(&[])
                   .unwrap_err()
                   .to_string(),
               HyperError::Header.to_string());
    assert_eq!(RawFsApiHeader::parse_header(&[vec![]])
                   .unwrap_err()
                   .to_string(),
               HyperError::Header.to_string());
    assert_eq!(RawFsApiHeader::parse_header(&[vec![b'1', b'0']])
                   .unwrap_err()
                   .to_string(),
               HyperError::Header.to_string());
    assert_eq!(RawFsApiHeader::parse_header(&[vec![b'1'], vec![b'1']])
                   .unwrap_err()
                   .to_string(),
               HyperError::Header.to_string());
}

#[test]
fn fmt_header() {
    assert_eq!(&HeaderFormatTester(RawFsApiHeader(true)).to_string(), "1");
    assert_eq!(&HeaderFormatTester(RawFsApiHeader(false)).to_string(), "0");
}


struct HeaderFormatTester<T: HeaderFormat>(T);

impl<T: HeaderFormat> fmt::Display for HeaderFormatTester<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt_header(f)
    }
}
