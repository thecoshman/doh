use self::super::util::{human_readable_size, parent_url, GETCH_ARROW_PREFIX, GETCH_ARROW_RIGHT, GETCH_ARROW_LEFT, GETCH_ARROW_DOWN, GETCH_ARROW_UP, TAB_SPACING,
                        GETCH_ENTER, USER_AGENT, GETCH_ESC};
use rfsapi::{RawFsApiHeader, FilesetData, RawFileData};
use std::io::{self, BufReader, BufRead, Write, Read};
use reqwest::{Response, IntoUrl, Client, Url};
use reqwest::header::UserAgent;
use itertools::Itertools;
use tabwriter::TabWriter;
use std::{cmp, fmt};
use getch::Getch;
use time::Tm;

pub mod term;


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


/// Copy `label`led data from `reader` to `writer`, stopping each screen, wrapping at `term_size`,
/// polling `input` for instructions.
///
/// Never panics, returns `Ok(true)` if paging succeeded or `Ok(false)` if paged data isn't UTF-8.
///
/// ### Controls on nonlast pages
///
/// Key|Result
/// ---|------
/// Escape | stop paging
/// Any key | go to next page
///
/// Controls on last page:
///
/// Key|Result
/// ---|------
/// Any key | stop paging
pub fn paging_copy<R: Read, W: Write>(reader: &mut R, writer: &mut W, label: &str, input: &Getch, term_size: (usize, usize)) -> io::Result<()> {
    const END_MESSAGE_RAW_LEN: usize = 31 + 4 + 22 + 2 + 1 + 2 + 1;

    let (tx, ty) = term_size;
    let mut reader = BufReader::new(reader);
    let mut outlines: Vec<String> = vec![];

    let mut line = String::new();
    while try!(reader.read_line(&mut line)) != 0 {
        line = line.replace(&['\r', '\n'][..], "").replace('\t', &TAB_SPACING);
        if line.is_empty() {
            outlines.push(String::new());
        } else {
            outlines.extend(line.drain(..).chunks(tx).into_iter().map(|l| l.collect()));
        }
    }
    let _ = line;

    let end_message_len = END_MESSAGE_RAW_LEN + label.chars().count();
    let end_message_lines = (end_message_len as f64 / tx as f64).floor() as usize;
    let screen_count = (outlines.len() as f64 / (ty - end_message_lines) as f64).ceil() as usize;
    for (si, screen) in outlines.chunks(ty - end_message_lines).enumerate() {
        if si != 0 {
            try!(writeln!(writer));
        }
        for line in screen {
            try!(writeln!(writer, "{}", line));
        }

        try!(write!(writer, "<{}> ", label));
        if si == screen_count - 1 {
            try!(write!(writer, "<End of file> <Press any key to stop>"));
        } else {
            try!(write!(writer,
                        "<{}%> <Press any key for {} screen> <Press Escape to stop>",
                        ((si + 1) as f64 / screen_count as f64 * 100f64).round(),
                        if si + 1 == screen_count - 1 {
                            "last"
                        } else {
                            "next"
                        }));
        }
        try!(writer.flush());

        if input.getch().unwrap() == GETCH_ESC {
            break;
        }
    }

    try!(writeln!(writer));
    Ok(())
}


/// Main context used to list a server.
pub struct ListContext {
    cururl: Url,
    files: Vec<RemoteFile>,
    selected: usize,
}

impl ListContext {
    /// Create a context, starting off at the provided URL.
    pub fn new(starting_url: Url) -> ListContext {
        ListContext {
            cururl: starting_url,
            files: vec![],
            selected: 0,
        }
    }

    /// Do one run of the input loop, effectively resulting in a single action.
    ///
    /// Returns `Ok(true)` to allow to continue next loop or `Ok(false)` to end listing.
    ///
    /// ### Commands
    ///
    /// Key|Result
    /// -------|-------
    /// Enter/Right Arrow | enter highlighted entry
    /// Escape/`'Q'`/`'q'` | end
    /// Up Arrow | move selection 1 entry up
    /// Down Arrow | move selection 1 entry down
    /// Left Arrow | go up one level, if not at root
    ///
    /// ### Entering entries
    ///
    /// If the entry is a directory, on the next loop the selected subdirectory wll be listed.
    ///
    /// If the entry is a file, its contents are paged, see [`paging_copy()`](fn.paging_copy.html).
    ///
    /// TODO: Non-UTF-8 file support.
    pub fn one_loop<W: Write>(&mut self, mut out: &mut W, input: &Getch, term_size: (usize, usize)) -> io::Result<bool> {
        try!(writeln!(out, "Contents of {}:", self.cururl));
        let mut resp = download(self.cururl.clone());
        if !resp.status().is_success() {
            try!(writeln!(out, "<Got {}...>", resp.status()));
            self.back();
            return Ok(true);
        }

        let data = match resp.json::<FilesetData>() {
            Ok(d) => d,
            Err(e) => {
                try!(writeln!(out, "<Couldn't parse server response: {}...>", e));
                self.back();
                return Ok(true);
            }
        };

        if data.is_file {
            try!(paging_copy(&mut download_raw(self.cururl.clone()), out, &self.cururl.path()[1..], &input, term_size));
            self.cururl = parent_url(&self.cururl);
        } else {
            self.files = RemoteFile::from_response(data);
            {
                let mut tout = TabWriter::new(&mut out);
                for (i, f) in self.files.iter().enumerate() {
                    try!(writeln!(tout, "{}{}", if i == self.selected { ">" } else { " " }, f));
                }
                try!(tout.flush());
            }

            while let (true, exit) = try!(self.process_input(out, input)) {
                if exit {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    fn process_input<W: Write>(&mut self, out: &mut W, input: &Getch) -> io::Result<(bool, bool)> {
        match try!(input.getch()) {
            GETCH_ENTER => Ok((self.select(), false)),
            GETCH_ESC | b'q' | b'Q' => Ok((true, true)),
            GETCH_ARROW_PREFIX => {
                match try!(input.getch()) {
                    GETCH_ARROW_UP => {
                        if self.selected != 0 {
                            try!(self.update_selected(out, ' '));
                            self.selected -= 1;
                            try!(self.update_selected(out, '>'));
                        }
                        Ok((true, false))
                    }
                    GETCH_ARROW_DOWN => {
                        try!(self.update_selected(out, ' '));
                        self.selected = cmp::min(self.selected + 1, self.files.len() - 1);
                        try!(self.update_selected(out, '>'));
                        Ok((true, false))
                    }
                    GETCH_ARROW_LEFT => Ok((self.back(), false)),
                    GETCH_ARROW_RIGHT => Ok((self.select(), false)),
                    _ => Ok((true, false)),
                }
            }
            _ => Ok((true, false)),
        }
    }

    fn update_selected<W: Write>(&self, out: &mut W, c: char) -> io::Result<()> {
        try!(write!(out, "{}", term::move_cursor_up(self.files.len() - self.selected)));
        try!(write!(out, "{}", c));
        try!(out.flush());
        try!(write!(out, "{}", term::move_cursor_back(1)));
        try!(write!(out, "{}", term::move_cursor_down(self.files.len() - self.selected)));
        Ok(())
    }

    fn select(&mut self) -> bool {
        if !self.files.is_empty() {
            self.cururl = self.cururl.join(&self.files[self.selected].full_name).unwrap();
            self.selected = 0;
        }
        self.files.is_empty()
    }

    fn back(&mut self) -> bool {
        let parent = parent_url(&self.cururl);
        if parent != self.cururl {
            self.cururl = parent;
            self.selected = 0;
            true
        } else {
            false
        }
    }
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
    /// # extern crate rfsapi;
    /// # extern crate doh;
    /// # use doh::ops::RemoteFile;
    /// # use doh::util::parse_rfc3339;
    /// # use rfsapi::{FilesetData, RawFileData};
    /// # fn main() {
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
    /// # }
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
