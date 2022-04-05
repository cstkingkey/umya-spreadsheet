use std::fs::File;
use std::io;
use std::path::Path;
use std::string::FromUtf8Error;
use std::sync::Arc;
use std::sync::RwLock;

use super::driver;
use structs::raw::RawWorksheet;
use structs::SharedStringTable;
use structs::Spreadsheet;
use structs::Stylesheet;
use structs::Theme;
use structs::Worksheet;

pub(crate) mod chart;
pub(crate) mod comment;
mod content_types;
mod doc_props_app;
mod doc_props_core;
pub(crate) mod drawing;
mod rels;
mod shared_strings;
mod styles;
mod theme;
mod vba_project_bin;
pub(crate) mod vml_drawing;
mod workbook;
mod workbook_rels;
pub(crate) mod worksheet;

#[derive(Debug)]
pub enum XlsxError {
    Io(io::Error),
    Xml(quick_xml::Error),
    Zip(zip::result::ZipError),
    Uft8(FromUtf8Error),
}

impl From<io::Error> for XlsxError {
    fn from(err: io::Error) -> XlsxError {
        XlsxError::Io(err)
    }
}

impl From<quick_xml::Error> for XlsxError {
    fn from(err: quick_xml::Error) -> XlsxError {
        XlsxError::Xml(err)
    }
}

impl From<zip::result::ZipError> for XlsxError {
    fn from(err: zip::result::ZipError) -> XlsxError {
        XlsxError::Zip(err)
    }
}

impl From<FromUtf8Error> for XlsxError {
    fn from(err: FromUtf8Error) -> XlsxError {
        XlsxError::Uft8(err)
    }
}

/// read spreadsheet from arbitrary reader.
/// # Arguments
/// * `reader` - reader to read from.
/// # Return value
/// * `Result` - OK is Spreadsheet. Err is error message.
pub fn read_reader<R: io::Read + io::Seek>(
    reader: R,
    with_sheet_read: bool,
) -> Result<Spreadsheet, XlsxError> {
    let mut arv = zip::read::ZipArchive::new(reader)?;

    let mut book = workbook::read(&mut arv).unwrap();
    doc_props_app::read(&mut arv, &mut book).unwrap();
    doc_props_core::read(&mut arv, &mut book).unwrap();
    vba_project_bin::read(&mut arv, &mut book).unwrap();
    content_types::read(&mut arv, &mut book).unwrap();
    let workbook_rel = workbook_rels::read(&mut arv, &mut book).unwrap();

    for (_, type_value, rel_target) in &workbook_rel {
        match type_value.as_str() {
            "http://schemas.openxmlformats.org/officeDocument/2006/relationships/theme" => {
                let theme = theme::read(&mut arv, rel_target).unwrap();
                book.set_theme(theme);
            }
            _ => {}
        }
    }
    let _theme = book.get_theme().clone();

    shared_strings::read(&mut arv, &mut book).unwrap();
    styles::read(&mut arv, &mut book).unwrap();

    for sheet in book.get_sheet_collection_mut() {
        for (rel_id, _, rel_target) in &workbook_rel {
            if sheet.get_r_id() != rel_id {
                continue;
            }
            let mut raw_worksheet = RawWorksheet::default();
            raw_worksheet.read(&mut arv, &rel_target);
            sheet.set_raw_data_of_worksheet(raw_worksheet);
        }
    }

    if with_sheet_read {
        book.read_sheet_collection();
    }

    Ok(book)
}

/// read spreadsheet file.
/// # Arguments
/// * `path` - file path to read.
/// # Return value
/// * `Result` - OK is Spreadsheet. Err is error message.
/// # Examples
/// ```
/// let path = std::path::Path::new("./tests/test_files/aaa.xlsx");
/// let mut book = umya_spreadsheet::reader::xlsx::read(path).unwrap();
/// ```
pub fn read(path: &Path) -> Result<Spreadsheet, XlsxError> {
    let file = File::open(path)?;
    read_reader(file, true)
}

/// lazy read spreadsheet file.
/// Delays the loading of the worksheet until it is needed.
/// When loading a file with a large amount of data, response improvement can be expected.
/// # Arguments
/// * `path` - file path to read.
/// # Return value
/// * `Result` - OK is Spreadsheet. Err is error message.
/// # Examples
/// ```
/// let path = std::path::Path::new("./tests/test_files/aaa.xlsx");
/// let mut book = umya_spreadsheet::reader::xlsx::lazy_read(path).unwrap();
/// ```
pub fn lazy_read(path: &Path) -> Result<Spreadsheet, XlsxError> {
    let file = File::open(path)?;
    read_reader(file, false)
}

pub(crate) fn raw_to_serialize_by_worksheet(
    worksheet: &mut Worksheet,
    theme: &Theme,
    shared_string_table: Arc<RwLock<SharedStringTable>>,
    stylesheet: &Stylesheet,
) {
    if worksheet.is_serialized() {
        return;
    }

    let raw_data_of_worksheet = worksheet.get_raw_data_of_worksheet().clone();
    let shared_string_table = &*shared_string_table.read().unwrap();
    worksheet::read(
        worksheet,
        &raw_data_of_worksheet,
        theme,
        &shared_string_table,
        stylesheet,
    )
    .unwrap();

    match raw_data_of_worksheet.get_worksheet_relationships() {
        Some(v) => {
            for relationship in v.get_relationship_list() {
                match relationship.get_type() {
                    // drawing, chart
                    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/drawing" => {
                        drawing::read(
                            worksheet,
                            relationship.get_raw_file(),
                            raw_data_of_worksheet.get_drawing_relationships(),
                        )
                        .unwrap();
                    }
                    // comment
                    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/comments" => {
                        let _ = comment::read(worksheet, relationship.get_raw_file()).unwrap();
                    }
                    _ => {}
                }
            }
            for relationship in v.get_relationship_list() {
                match relationship.get_type() {
                    // vmlDrawing
                    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/vmlDrawing" => {
                        let _ = vml_drawing::read(
                            worksheet,
                            relationship.get_raw_file(),
                            raw_data_of_worksheet.get_vml_drawing_relationships(),
                        )
                        .unwrap();
                    }
                    _ => {}
                }
            }
        }
        None => {}
    }

    worksheet.remove_raw_data_of_worksheet();
}
