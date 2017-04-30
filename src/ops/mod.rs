use util::{human_readable_size, USER_AGENT};
use reqwest::{Response, IntoUrl, Client};
use reqwest::header::UserAgent;
use time::Tm;
use std::fmt;

mod rfsapi;

pub use self::rfsapi::{RawFsApiHeader, FilesetData, RawFileData};


/// GET a resource with the RFSAPI header, auto-unpacking GZip.
pub fn download<U: IntoUrl>(u: U) -> Response {
    really_download(u, true)
}

/// GET a resource normally, auto-unpacking GZip.
pub fn download_raw<U: IntoUrl>(u: U) -> Response {
    really_download(u, false)
}

fn really_download<U: IntoUrl>(u: U, raw: bool) -> Response {
    let mut client = Client::new().unwrap();
    client.gzip(true);
    client.get(u)
        .header(RawFsApiHeader(raw))
        .header(UserAgent(USER_AGENT.to_string()))
        .send()
        .unwrap()
}


/// Parsed `RawFileData` entry, prepared for end-user usage.
#[derive(Eq, PartialEq, Ord, PartialOrd, Debug, Clone, Hash)]
pub struct RemoteFile {
    /// Resource's name, ends with `'/'` if it's a directory.
    pub full_name: String,
    /// File size in bytes, `None` for directories.
    pub size: Option<u64>,
    /// Human-readable file size, `None` for directories.
    ///
    /// See [`util::human_readable_size()`](../util/fn.human_readable_size.html).
    pub human_size: Option<String>,
    /// Last modification time in local TZ, `None` for special `"../"` entry.
    pub last_modified: Option<Tm>,
}

impl RemoteFile {
    /// Parse a server-returned fileset into a set of user-viewable files.
    ///
    /// The returned vector is empty if the specified fileset describes a file.
    ///
    /// The returned vector contains an additional entry at the front with name `"../"` if the fileset isn't a root directory.
    ///
    /// The returned vector is partitioned according depending on directoriness status of the entry, directories first.
    ///
    /// # Examples
    ///
    /// ```
    /// # use doh::ops::{RemoteFile, FilesetData, RawFileData};
    /// # use doh::util::parse_rfc3339;
    /// assert_eq!(RemoteFile::from_response(FilesetData {
    ///     writes_supported: false,
    ///     is_root: false,
    ///     is_file: false,
    ///     files: vec![
    ///         RawFileData {
    ///             mime_type: "text/directory".parse().unwrap(),
    ///             name: "data".to_string(),
    ///             last_modified: parse_rfc3339("2012-02-22T14:53:18.42Z").unwrap(),
    ///             size: 0,
    ///             is_file: false,
    ///         },
    ///         RawFileData {
    ///             mime_type: "text/html".parse().unwrap(),
    ///             name: "index.html".to_string(),
    ///             last_modified: parse_rfc3339("2012-02-22T15:23:18Z").unwrap(),
    ///             size: 2297,
    ///             is_file: true,
    ///         },
    ///     ],
    /// }), vec![
    ///     RemoteFile {
    ///         full_name: "../".to_string(),
    ///         size: None,
    ///         human_size: None,
    ///         last_modified: None,
    ///     },
    ///     RemoteFile {
    ///         full_name: "data/".to_string(),
    ///         size: None,
    ///         human_size: None,
    ///         last_modified: Some(parse_rfc3339("2012-02-22T14:53:18.42Z").unwrap().to_local()),
    ///     },
    ///     RemoteFile {
    ///         full_name: "index.html".to_string(),
    ///         size: Some(2297),
    ///         human_size: Some("2.2KiB".to_string()),
    ///         last_modified: Some(parse_rfc3339("2012-02-22T15:23:18Z").unwrap().to_local()),
    ///     },
    /// ]);
    /// ```
    pub fn from_response(mut resp: FilesetData) -> Vec<RemoteFile> {
        if resp.is_file {
            vec![]
        } else {
            resp.files.sort_by_key(|f| f.is_file);
            let mut fs: Vec<_> = resp.files
                .into_iter()
                .map(From::from)
                .collect();
            if !resp.is_root {
                fs.insert(0,
                          RemoteFile {
                              full_name: "../".to_string(),
                              size: None,
                              human_size: None,
                              last_modified: None,
                          });
            }
            fs
        }
    }
}

impl From<RawFileData> for RemoteFile {
    fn from(rfd: RawFileData) -> RemoteFile {
        RemoteFile {
            full_name: rfd.name + if rfd.is_file { "" } else { "/" },
            size: if rfd.is_file { Some(rfd.size) } else { None },
            human_size: if rfd.is_file {
                Some(human_readable_size(rfd.size))
            } else {
                None
            },
            last_modified: Some(rfd.last_modified.to_local()),
        }
    }
}

/// Designed for use with `TabWriter`, separates entries with `\t`s.
impl fmt::Display for RemoteFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(f, "{}\t", self.full_name));
        if let Some(ref hs) = self.human_size.as_ref() {
            try!(write!(f, "{}", hs));
        }
        try!(write!(f, "\t"));
        if let Some(ref lm) = self.last_modified.as_ref() {
            try!(write!(f, "{}", lm.rfc3339()));
        }
        Ok(())
    }
}
