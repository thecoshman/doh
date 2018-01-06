use self::super::util::{human_readable_size, max_listing_lines, percent_decode, parent_url, GETCH_SPECIAL_PREFIX, GETCH_ARROW_RIGHT, GETCH_ARROW_LEFT,
                        GETCH_ARROW_DOWN, GETCH_ARROW_UP, GETCH_PAGE_DOWN, GETCH_PAGE_UP, TAB_SPACING, GETCH_DELETE, GETCH_ENTER, USER_AGENT, GETCH_HOME,
                        GETCH_ESC, GETCH_END};
use rfsapi::{RawFsApiHeader, FilesetData, RawFileData};
use std::io::{self, BufReader, BufRead, Write, Read};
use reqwest::{ClientBuilder, Response, IntoUrl, Client, Url};
use pbr::{Units as ProgressBarUnits, ProgressBar};
use reqwest::header::{ContentLength, UserAgent};
use std::path::{PathBuf, Path};
use itertools::Itertools;
use tabwriter::TabWriter;
use std::time::Duration;
use std::fs::File;
use getch::Getch;
use std::fmt;
use time::Tm;

pub mod term;


/// PUT a resource.
pub fn upload<U: IntoUrl>(u: U, f: File) -> Response {
    client().put(u).header(UserAgent::new(USER_AGENT)).body(f).send().unwrap()
}

/// DELETE a resource.
pub fn delete<U: IntoUrl>(u: U) -> Response {
    client().delete(u).header(UserAgent::new(USER_AGENT)).send().unwrap()
}

/// GET a resource with the RFSAPI header, auto-unpacking gzip.
pub fn download<U: IntoUrl>(u: U) -> Response {
    really_download(u, true)
}

/// GET a resource normally, auto-unpacking gzip.
pub fn download_raw<U: IntoUrl>(u: U) -> Response {
    really_download(u, false)
}

fn really_download<U: IntoUrl>(u: U, raw: bool) -> Response {
    client().get(u).header(RawFsApiHeader(raw)).header(UserAgent::new(USER_AGENT)).send().unwrap()
}

fn client() -> Client {
    ClientBuilder::new().gzip(true).build().unwrap()
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
pub fn paging_copy<R: Read, W: Write>(reader: &mut R, writer: &mut W, label: &str, input: &Getch, term_size: (usize, usize)) -> io::Result<bool> {
    /// Costant width of screen-end message for wrapping, as seen in-code from bottom right to top left.
    const END_MESSAGE_RAW_LEN: usize = 31 + 4 + 22 + 2 + 1 + 2 + 1;

    let (tx, ty) = term_size;
    let mut reader = BufReader::new(reader);
    let mut outlines: Vec<String> = vec![];

    let mut line = String::new();
    while match reader.read_line(&mut line) {
        Ok(read) => read != 0,
        Err(e) => {
            return if e.kind() == io::ErrorKind::InvalidData {
                Ok(false)
            } else {
                Err(e)
            };
        }
    } {
        line = line.replace(&['\r', '\n'][..], "").replace('\t', &TAB_SPACING);
        if line.is_empty() {
            outlines.push(String::new());
        } else {
            outlines.extend(line.drain(..).chunks(tx).into_iter().map(|l| l.collect()));
        }
    }
    drop(line);

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

        match try!(input.getch()) {
            GETCH_ESC => break,
            GETCH_SPECIAL_PREFIX => {
                // Prevent doublescroll when using arrows
                try!(input.getch());
            }
            _ => {}
        }
    }

    try!(writeln!(writer));
    Ok(true)
}


/// Main context used to list a server.
pub struct ListContext {
    cururl: Url,
    files: Vec<RemoteFile>,
    selected: usize,
    have_write: bool,
    bad_response_counter: usize,
}

impl ListContext {
    /// Create a context, starting off at the provided URL.
    pub fn new(starting_url: Url) -> ListContext {
        ListContext {
            cururl: starting_url,
            files: vec![],
            selected: 0,
            have_write: false,
            bad_response_counter: 0,
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
    /// `'D'`/`'d'` | download file
    /// `'U'`/`'u'` | upload file
    /// Up Arrow | move selection 1 entry up
    /// Down Arrow | move selection 1 entry down
    /// Left Arrow | go up one level, if not at root
    /// Home | set selection to first item
    /// End | set selection to last item
    /// Page Up | move selection page's worth of entries up
    /// Page Down | move selection page's worth of entries down
    /// Delete | `DELETE` highlighted entry
    ///
    /// ### Entering entries
    ///
    /// If the entry is a directory, on the next loop the selected subdirectory wll be listed.
    ///
    /// If the entry is a UTF-8 file, its contents are paged, see [`paging_copy()`](fn.paging_copy.html).
    ///
    /// If the entry is a non-UTF-8 file, it is [downloaded](#downloading-files).
    ///
    /// ### Downloading files
    ///
    /// The user is shown a file picker and let choose where to download the file to,
    /// then the file is downloaded and saved to the specified location.
    ///
    /// The file isn't saved if the user cancels the picker.
    ///
    /// A progress bar is shown to the user indicating the download progress.
    ///
    /// ### Uploading files
    ///
    /// If the server, in the last RFSAPI request, specified `writes_supported` as `false`, an error is printed,
    /// and nothing happens.
    ///
    /// Otherwise:
    ///
    /// The user is shown a file picker and let choose which file to upload,
    /// then the file is streamed to the server via a `PUT` request in the currently selected directory,
    /// with the name of the picked file.
    ///
    /// The file isn't uploaded if the user cancels the picker.
    pub fn one_loop<W: Write>(&mut self, out: &mut W, input: &Getch, term_size: (usize, usize)) -> io::Result<bool> {
        let mut resp = download(self.cururl.clone());
        if !resp.status().is_success() {
            try!(writeln!(out, "Contents of {}:", percent_decode(&self.cururl.to_string()).unwrap()));
            try!(writeln!(out, "<Got {}...>", resp.status()));
            self.back();
            return Ok(true);
        }

        let data = match resp.json::<FilesetData>() {
            Ok(d) => d,
            Err(e) => {
                try!(writeln!(out, "Contents of {}:", percent_decode(&self.cururl.to_string()).unwrap()));
                try!(writeln!(out, "<Couldn't parse server response: {}...>", e));
                self.bad_response_counter += !self.back() as usize;
                if self.bad_response_counter == 3 {
                    try!(writeln!(out, "<Server at {} doesn't support RFSAPI.>", self.cururl));
                }
                return Ok(self.bad_response_counter != 3);
            }
        };
        self.have_write = data.writes_supported;

        if data.is_file {
            if !try!(paging_copy(&mut download_raw(self.cururl.clone()), out, &self.cururl.path()[1..], input, term_size)) {
                try!(writeln!(out, "Contents of {}:", percent_decode(&self.cururl.to_string()).unwrap()));
                try!(writeln!(out, "<Not UTF-8, select download destination>"));
                try!(self.download_file(out, self.cururl.clone(), term_size));
            }
            self.cururl = parent_url(&self.cururl);
        } else {
            self.files = RemoteFile::from_response(data);

            try!(self.list_file_screen(out, term_size));
            while let (true, exit) = try!(self.process_input(out, input, term_size)) {
                if exit {
                    return Ok(false);
                }
            }
        }

        Ok(true)
    }

    fn list_file_screen<W: Write>(&self, mut out: &mut W, term_size: (usize, usize)) -> io::Result<()> {
        let (_, ty) = term_size;
        let lines_per_screen = max_listing_lines(ty);
        let cur_screen = self.selected / lines_per_screen;
        let screen_count = (self.files.len() as f64 / lines_per_screen as f64).ceil() as usize;

        try!(writeln!(out,
                      "Contents of {} -- page {}/{}:",
                      percent_decode(&self.cururl.to_string()).unwrap(),
                      cur_screen + 1,
                      screen_count));

        let mut tout = TabWriter::new(&mut out);
        for (i, f) in self.files.iter().enumerate().skip(cur_screen * lines_per_screen).take(lines_per_screen) {
            try!(writeln!(tout, "{}{}", if i == self.selected { ">" } else { " " }, f));
        }
        try!(tout.flush());

        Ok(())
    }

    fn process_input<W: Write>(&mut self, out: &mut W, input: &Getch, term_size: (usize, usize)) -> io::Result<(bool, bool)> {
        match try!(input.getch()) {
            GETCH_ENTER => Ok((self.select(), false)),
            GETCH_ESC | b'q' | b'Q' => Ok((true, true)),
            b'd' | b'D' => {
                let download_ok = !self.files.is_empty() && self.files[self.selected].size.is_some();
                if download_ok {
                    try!(self.download_file(out, self.cururl.join(&self.files[self.selected].full_name).unwrap(), term_size));
                }
                Ok((!download_ok, false))
            }
            b'u' | b'U' => {
                if self.have_write {
                    try!(self.upload(out));
                } else {
                    try!(writeln!(out, "<Server doesn't permit write requests>"));
                }
                Ok((false, false))
            }
            GETCH_SPECIAL_PREFIX => {
                let (_, ty) = term_size;
                let lines_per_screen = max_listing_lines(ty);

                match try!(input.getch()) {
                    GETCH_ARROW_UP => {
                        if self.selected != 0 {
                            try!(self.update_selected(out, ' ', term_size));
                            self.selected -= 1;
                            if self.selected != 0 && (self.selected + 1) % lines_per_screen == 0 {
                                try!(self.list_file_screen(out, term_size));
                            }
                            try!(self.update_selected(out, '>', term_size));
                        }
                        Ok((true, false))
                    }
                    GETCH_ARROW_DOWN => {
                        if self.selected + 1 != self.files.len() {
                            try!(self.update_selected(out, ' ', term_size));
                            self.selected += 1;
                            if self.selected % lines_per_screen == 0 {
                                try!(self.list_file_screen(out, term_size));
                            }
                            try!(self.update_selected(out, '>', term_size));
                        }
                        Ok((true, false))
                    }
                    GETCH_PAGE_UP => {
                        try!(self.update_selected(out, ' ', term_size));
                        if self.selected >= lines_per_screen {
                            self.selected -= lines_per_screen;
                            try!(self.list_file_screen(out, term_size));
                        } else {
                            self.selected = 0;
                        }
                        try!(self.update_selected(out, '>', term_size));
                        Ok((true, false))
                    }
                    GETCH_PAGE_DOWN => {
                        try!(self.update_selected(out, ' ', term_size));
                        if self.selected + lines_per_screen < self.files.len() {
                            self.selected += lines_per_screen;
                            try!(self.list_file_screen(out, term_size));
                        } else {
                            self.selected = self.files.len() - 1;
                        }
                        try!(self.update_selected(out, '>', term_size));
                        Ok((true, false))
                    }
                    GETCH_HOME => {
                        let selecteded = self.selected;

                        try!(self.update_selected(out, ' ', term_size));
                        self.selected = 0;
                        if selecteded >= lines_per_screen {
                            try!(self.list_file_screen(out, term_size));
                        }
                        try!(self.update_selected(out, '>', term_size));
                        Ok((true, false))
                    }
                    GETCH_END => {
                        let selecteded = self.selected;
                        let screen_count = (self.files.len() as f64 / lines_per_screen as f64).ceil() as usize;

                        try!(self.update_selected(out, ' ', term_size));
                        self.selected = self.files.len() - 1;
                        if selecteded < (screen_count - 2) * lines_per_screen {
                            try!(self.list_file_screen(out, term_size));
                        }
                        try!(self.update_selected(out, '>', term_size));
                        Ok((true, false))
                    }
                    GETCH_ARROW_LEFT => Ok((self.back(), false)),
                    GETCH_ARROW_RIGHT => Ok((self.select(), false)),
                    GETCH_DELETE => {
                        if self.have_write {
                            try!(self.delete(out));
                        } else {
                            try!(writeln!(out, "<Server doesn't permit write requests>"));
                        }
                        Ok((false, false))
                    }
                    _ => Ok((true, false)),
                }
            }
            _ => Ok((true, false)),
        }
    }

    fn update_selected<W: Write>(&self, out: &mut W, c: char, term_size: (usize, usize)) -> io::Result<()> {
        let (_, ty) = term_size;
        let lines_per_screen = max_listing_lines(ty);
        let cur_screen = self.selected / lines_per_screen;
        let screen_count = (self.files.len() as f64 / lines_per_screen as f64).ceil() as usize;
        let lines_this_screen = if cur_screen == screen_count - 1 {
            self.files.len() % lines_per_screen
        } else {
            lines_per_screen
        };
        let delta_h = lines_this_screen - (self.selected - (cur_screen * lines_per_screen));

        try!(write!(out, "{}", term::move_cursor_up(delta_h)));
        try!(write!(out, "{}", c));
        try!(out.flush());
        try!(write!(out, "{}", term::move_cursor_back(1)));
        try!(write!(out, "{}", term::move_cursor_down(delta_h)));

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

    fn download_file<W: Write>(&self, mut out: &mut W, u: Url, term_size: (usize, usize)) -> io::Result<()> {
        let f = PathBuf::from(&u.path()[1..]);
        if let Some(outp) = term::save_file_picker(f.file_name().unwrap(), f.extension()) {
            try!(writeln!(out, "<Downloading to {}...>", outp.display()));

            let mut resp = download_raw(u);
            let mut outf = try!(File::create(&outp));
            if let Some(size) = resp.headers().get::<ContentLength>().map(|cl| cl.0) {
                let (tx, _) = term_size;
                let mut pb = ProgressBar::on(&mut out, size);
                pb.set_units(ProgressBarUnits::Bytes);
                pb.set_width(Some(tx));
                pb.set_max_refresh_rate(Some(Duration::from_secs(1)));

                let mut buf = vec![0; 4096];
                loop {
                    let read = resp.read(&mut buf[..]).unwrap();
                    if read == 0 {
                        break;
                    }

                    try!(outf.write_all(&buf));
                    pb.add(read as u64);
                }

                pb.finish_println("");
            } else {
                try!(io::copy(&mut resp, &mut outf));
            }

            try!(writeln!(out, "<Done!>"));
        }
        Ok(())
    }

    fn upload<W: Write>(&self, out: &mut W) -> io::Result<()> {
        if let Some(inp) = term::open_file_picker() {
            try!(writeln!(out, "<Uploading {} to {}...>", inp.display(), percent_decode(&self.cururl.to_string()).unwrap()));
            let upurl = self.cururl.join(&Path::new(inp.file_name().unwrap()).display().to_string()).unwrap();
            let status = upload(upurl, try!(File::open(inp))).status();
            if status.is_success() {
                try!(writeln!(out, "<Success!>"));
            } else {
                try!(writeln!(out, "<Got {}...>", status));
            }
        }
        Ok(())
    }

    fn delete<W: Write>(&mut self, out: &mut W) -> io::Result<()> {
        let delurl = self.cururl.join(&self.files[self.selected].full_name).unwrap();
        try!(writeln!(out, "<Deleting {}...>", percent_decode(&delurl.to_string()).unwrap()));
        let status = delete(delurl.clone()).status();
        if status.is_success() {
            try!(writeln!(out, "<Success!>"));
        } else {
            try!(writeln!(out, "<Got {}...>", status));
        }
        self.selected = 0;
        Ok(())
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
        if let Some(hs) = self.human_size.as_ref() {
            try!(write!(f, "{}", hs));
        }
        try!(write!(f, "\t"));
        if let Some(lm) = self.last_modified.as_ref() {
            try!(write!(f, "{}", lm.strftime("%F %T").unwrap()));
        }
        Ok(())
    }
}
