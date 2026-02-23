use std::num::NonZero;

use anyhow::{Result, anyhow, bail};
use unicode_width::UnicodeWidthChar;

use crate::winget::PackageEntry;

pub fn parse_package_entries(output: &str) -> Result<Vec<PackageEntry>> {
    let mut lines = output.lines();
    let header = lines.next().ok_or_else(|| anyhow!("Invalid output"))?;
    let header = header.split('\r').next_back();
    let header = header.ok_or_else(|| anyhow!("Invalid output"))?;
    let header = parse_header(header);
    if &header[0].0 != "Name"
        || &header[1].0 != "Id"
        || &header[2].0 != "Version"
        || &header[3].0 != "Available"
        || &header[4].0 != "Source"
    {
        bail!("Invalid header");
    }
    let header: Vec<_> = header.iter().map(|&(_, len)| len).collect();
    lines.next().ok_or_else(|| anyhow!("Invalid output"))?;
    let entries = lines
        .map(|line| parse_entry(line, &header))
        .map(|mut columns| {
            let source = if columns.len() < 5 || columns[4].is_empty() {
                None
            } else {
                Some(columns.pop().unwrap().parse()?)
            };
            let available = if columns.len() < 4 {
                None
            } else {
                Some(columns.pop().unwrap()).filter(|x| !x.is_empty())
            };
            let version = columns.pop().unwrap();
            let id = columns.pop().unwrap();
            let name = columns.pop().unwrap();
            Ok(PackageEntry {
                source,
                id,
                _name: name,
                version,
                available,
            })
        })
        .collect::<Result<_>>()?;
    Ok(entries)
}

fn parse_header(text: &str) -> Vec<(String, Option<NonZero<usize>>)> {
    let mut columns = Vec::new();
    let mut start_idx = 0;
    let mut column_name = None;
    for (i, c) in text.chars().enumerate() {
        match column_name.is_some() {
            false => {
                if !c.is_ascii_graphic() {
                    column_name = Some(text.chars().skip(start_idx).take(i - start_idx).collect());
                }
            }
            true => {
                if c.is_ascii_graphic() {
                    columns.push((column_name.take().unwrap(), NonZero::new(i - start_idx)));
                    start_idx = i;
                }
            }
        }
    }
    columns.push((text.chars().skip(start_idx).collect(), None));
    columns
}

fn parse_entry(line: &str, headers: &[Option<NonZero<usize>>]) -> Vec<String> {
    let mut columns = Vec::new();
    let mut column_idx = 0;
    let mut count = 0;
    let mut start_idx = 0;
    for (i, c) in line.chars().enumerate() {
        count += c.width().unwrap_or(1);
        if let Some(column_len) = headers[column_idx]
            && count >= column_len.get()
        {
            count = 0;
            column_idx += 1;
            let column_value: String = line.chars().skip(start_idx).take(i - start_idx).collect();
            columns.push(column_value.trim().to_string());
            start_idx = i;
        }
    }
    let column_value: String = line.chars().skip(start_idx).collect();
    columns.push(column_value.trim().to_string());
    columns
}

#[cfg(test)]
mod tests {
    use crate::winget::Source;

    use super::*;

    #[test]
    fn test_parse_package_entries() {
        let entries = parse_package_entries(
            r"Name                                            Id                                        Version    Available  Source
-----------------------------------------------------------------------------------------------------------------------
PowerToys (Preview) x64                         Microsoft.PowerToys                       0.97.1     0.97.2     winget
Microsoft Visual C++ 2010  x64 Redistributable… Microsoft.VCRedist.2010.x64               10.0.40219            winget
PowerShell                                      9MZ1SNWT0N5D                              7.5.4.0               msstore
Windows Notepad                                 MSIX\Microsoft.WindowsNotepad_11.2510.14… 11.2510.1…
Windows ターミナル                              Microsoft.WindowsTerminal                 1.23.2021…            winget
MSYS2 64bit                                     MSYS2.MSYS2                               20220603   20251213   winget
",
        ).unwrap();
        assert_eq!(entries[0]._name, "PowerToys (Preview) x64".to_string());
        assert_eq!(entries[0].id, "Microsoft.PowerToys");
        assert_eq!(entries[0].source, Some(Source::Winget));
        assert_eq!(entries[0].version, "0.97.1");
        assert_eq!(entries[0].available, Some("0.97.2".to_string()));

        assert_eq!(
            entries[1]._name,
            "Microsoft Visual C++ 2010  x64 Redistributable…".to_string()
        );
        assert_eq!(entries[1].id, "Microsoft.VCRedist.2010.x64");
        assert_eq!(entries[1].source, Some(Source::Winget));
        assert_eq!(entries[1].version, "10.0.40219");
        assert_eq!(entries[1].available, None);

        assert_eq!(entries[2]._name, "PowerShell".to_string());
        assert_eq!(entries[2].id, "9MZ1SNWT0N5D");
        assert_eq!(entries[2].source, Some(Source::MsStore));
        assert_eq!(entries[2].version, "7.5.4.0");
        assert_eq!(entries[2].available, None);

        assert_eq!(entries[3]._name, "Windows Notepad".to_string());
        assert_eq!(entries[3].id, r"MSIX\Microsoft.WindowsNotepad_11.2510.14…");
        assert_eq!(entries[3].source, None);
        assert_eq!(entries[3].version, "11.2510.1…");
        assert_eq!(entries[3].available, None);

        assert_eq!(entries[4]._name, "Windows ターミナル".to_string());
        assert_eq!(entries[4].id, "Microsoft.WindowsTerminal");
        assert_eq!(entries[4].source, Some(Source::Winget));
        assert_eq!(entries[4].version, "1.23.2021…");
        assert_eq!(entries[4].available, None);

        assert_eq!(entries[5]._name, "MSYS2 64bit".to_string());
        assert_eq!(entries[5].id, "MSYS2.MSYS2");
        assert_eq!(entries[5].source, Some(Source::Winget));
        assert_eq!(entries[5].version, "20220603");
        assert_eq!(entries[5].available, Some("20251213".to_string()));
    }
}
