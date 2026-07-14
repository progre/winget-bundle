use std::{iter, num::NonZero};

use anyhow::{Context, Result};
use unicode_width::UnicodeWidthChar;

pub enum ColumnWidthBasis {
    Header,
    SeparatorLine,
}

pub fn parse_table<'a>(
    lines: impl Iterator<Item = &'a str>,
    column_width_basis: ColumnWidthBasis,
) -> Result<(usize, Vec<String>)> {
    let mut lines = lines.filter(|x| !x.is_empty());
    let mut line1 = lines.next().context("Invalid output")?;
    let mut line2: &str;
    loop {
        line2 = lines.next().context("Invalid output")?;
        if line2.starts_with("--") {
            break;
        }
        line1 = line2;
    }
    let header = line1;
    let separator_line = line2;
    let (header_len, cells) = match column_width_basis {
        ColumnWidthBasis::Header => {
            let header = parse_header(header);
            let header_len = header.len();
            let column_limits: Vec<_> = header
                .iter()
                .filter_map(|&(_, len)| len)
                .map(NonZero::get)
                .collect();
            let cells: Vec<_> = header
                .into_iter()
                .map(|(label, _)| label)
                .chain(lines.flat_map(|line| parse_entry(&column_limits, line)))
                .collect();
            (header_len, cells)
        }
        ColumnWidthBasis::SeparatorLine => {
            let separation_pos: Vec<_> = parse_separator_line(separator_line).collect();
            let column_limits = separation_pos
                .iter()
                .enumerate()
                .map(|(i, &pos)| {
                    if i == 0 {
                        pos
                    } else {
                        pos - separation_pos[i - 1]
                    }
                })
                .collect::<Vec<_>>();
            let cells: Vec<_> = parse_entry(&column_limits, header)
                .chain(lines.flat_map(|line| parse_entry(&column_limits, line)))
                .collect();
            (separation_pos.len() + 1, cells)
        }
    };
    debug_assert_eq!(cells.len() % header_len, 0);
    Ok((header_len, cells))
}

/// ヘッダー行から `(列名, 列幅)` を得る
///
/// `winget list` のヘッダー行は各列名が特定の表示列に揃えられ、次の列名の
/// 開始位置までスペースで埋められている。データ行も同じ表示列に揃う。
/// この時列幅は非 ASCII 列名を考慮し `UnicodeWidthChar` で数える。
/// 最終列の幅は `None`。データ行のパーサは最終列に残り全体を割り当てる。
fn parse_header(text: &str) -> Vec<(String, Option<NonZero<usize>>)> {
    // Pass 1: スペース区切りで列名を切り出しつつ、各列の開始表示列を控える。
    let mut columns: Vec<(String, usize)> = Vec::new();
    let mut current_name = String::new();
    let mut current_start = 0usize;
    let mut display_col = 0usize;
    for c in text.chars() {
        if c == ' ' {
            if !current_name.is_empty() {
                columns.push((std::mem::take(&mut current_name), current_start));
            }
        } else {
            if current_name.is_empty() {
                current_start = display_col;
            }
            current_name.push(c);
        }
        display_col += c.width().unwrap_or(0);
    }
    if !current_name.is_empty() {
        columns.push((current_name, current_start));
    }

    // Pass 2: 各列の幅を「次の列の開始位置 − 自分の開始位置」として算出する。
    // 最終列は次が無いので幅は `None`。
    let starts: Vec<usize> = columns.iter().map(|(_, s)| *s).collect();
    columns
        .into_iter()
        .enumerate()
        .map(|(i, (name, start))| {
            let width = starts
                .get(i + 1)
                .and_then(|&next| NonZero::new(next - start));
            (name, width)
        })
        .collect()
}

fn parse_separator_line(line: &str) -> impl Iterator<Item = usize> {
    let mut chars = line.chars().enumerate();
    let mut space = false;
    iter::from_fn(move || {
        for (i, c) in chars.by_ref() {
            match space {
                false => {
                    if c == ' ' {
                        space = true
                    }
                }
                true => {
                    if c != ' ' {
                        space = false;
                        return Some(i);
                    }
                }
            }
        }
        None
    })
}

// TODO: column_limits を separation_pos にする
fn parse_entry(column_limits: &[usize], line: &str) -> impl Iterator<Item = String> {
    let mut chars = line.chars().enumerate();
    let mut column_idx = 0;
    let mut column_len = 0;
    let mut start_idx = 0;
    iter::from_fn(move || {
        for (i, c) in chars.by_ref() {
            column_len += c.width().unwrap_or(1);
            if let Some(column_limit) = column_limits.get(column_idx)
                && column_len >= *column_limit
            {
                let len = i - start_idx;
                let column_value: String = line.chars().skip(start_idx).take(len).collect();
                column_idx += 1;
                column_len = 0;
                start_idx = i;
                return Some(column_value.trim().to_string());
            }
        }
        if column_idx <= column_limits.len() {
            let column_value: String = line.chars().skip(start_idx).collect();
            start_idx = line.len();
            column_idx += 1;
            return Some(column_value.trim().to_string());
        }
        None
    })
}

#[cfg(test)]
mod tests {
    use crate::package_manager::table_parser::{ColumnWidthBasis, parse_table};

    #[test]
    fn test_parse_package_entries_winget() {
        let (column_count, cells ) = parse_table(
            r"Name                                            Id                                        Version    Available  Source
-----------------------------------------------------------------------------------------------------------------------
PowerToys (Preview) x64                         Microsoft.PowerToys                       0.97.1     0.97.2     winget
Microsoft Visual C++ 2010  x64 Redistributable… Microsoft.VCRedist.2010.x64               10.0.40219            winget
PowerShell                                      9MZ1SNWT0N5D                              7.5.4.0               msstore
Windows Notepad                                 MSIX\Microsoft.WindowsNotepad_11.2510.14… 11.2510.1…
Windows ターミナル                              Microsoft.WindowsTerminal                 1.23.2021…            winget
MSYS2 64bit                                     MSYS2.MSYS2                               20220603   20251213   winget
"
            .lines(),
            ColumnWidthBasis::Header,
        )
        .unwrap();
        assert_eq!(column_count, 5);
        assert_eq!(
            cells[0..5],
            ["Name", "Id", "Version", "Available", "Source"]
        );
        assert_eq!(cells[5], "PowerToys (Preview) x64");
        assert_eq!(cells[6], "Microsoft.PowerToys");
        assert_eq!(cells[7], "0.97.1");
        assert_eq!(cells[8], "0.97.2");
        assert_eq!(cells[9], "winget");

        assert_eq!(cells[10], "Microsoft Visual C++ 2010  x64 Redistributable…");
        assert_eq!(cells[11], "Microsoft.VCRedist.2010.x64");
        assert_eq!(cells[12], "10.0.40219");
        assert_eq!(cells[13], "");
        assert_eq!(cells[14], "winget");

        assert_eq!(cells[15], "PowerShell".to_string());
        assert_eq!(cells[16], "9MZ1SNWT0N5D");
        assert_eq!(cells[17], "7.5.4.0");
        assert_eq!(cells[18], "");
        assert_eq!(cells[19], "msstore");

        assert_eq!(cells[20], "Windows Notepad".to_string());
        assert_eq!(cells[21], r"MSIX\Microsoft.WindowsNotepad_11.2510.14…");
        assert_eq!(cells[22], "11.2510.1…");
        assert_eq!(cells[23], "");
        assert_eq!(cells[24], "");

        assert_eq!(cells[25], "Windows ターミナル".to_string());
        assert_eq!(cells[26], "Microsoft.WindowsTerminal");
        assert_eq!(cells[27], "1.23.2021…");
        assert_eq!(cells[28], "");
        assert_eq!(cells[29], "winget");

        assert_eq!(cells[30], "MSYS2 64bit".to_string());
        assert_eq!(cells[31], "MSYS2.MSYS2");
        assert_eq!(cells[32], "20220603");
        assert_eq!(cells[33], "20251213");
        assert_eq!(cells[34], "winget");
    }

    #[test]
    fn test_parse_package_entries_winget_ja() {
        let (column_count, cells) = parse_table(
            r"名前                                                               ID                                                                                    バージョン           利用可能            ソース
---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------
PowerToys (Preview) x64                                            Microsoft.PowerToys                                                                   0.97.1               0.97.2              winget
Windows ターミナル                                                 Microsoft.WindowsTerminal                                                             1.23.2021…                               winget
"
            .lines(),
            ColumnWidthBasis::Header,
        )
        .unwrap();
        assert_eq!(column_count, 5);
        assert_eq!(cells[0], "名前");
        assert_eq!(cells[1], "ID");
        assert_eq!(cells[2], "バージョン");
        assert_eq!(cells[3], "利用可能");
        assert_eq!(cells[4], "ソース");
        assert_eq!(cells[5], "PowerToys (Preview) x64");
        assert_eq!(cells[6], "Microsoft.PowerToys");
        assert_eq!(cells[7], "0.97.1");
        assert_eq!(cells[8], "0.97.2");
        assert_eq!(cells[9], "winget");
        assert_eq!(cells[10], "Windows ターミナル");
        assert_eq!(cells[11], "Microsoft.WindowsTerminal");
        assert_eq!(cells[12], "1.23.2021…");
        assert_eq!(cells[13], "");
        assert_eq!(cells[14], "winget");
    }

    #[test]
    fn test_parse_package_entries_scoop() {
        let (column_count, cells) = parse_table(
            r"Name        Version      Source Updated             Info
----        -------      ------ -------             ----
7zip        26.00        main   2026-02-25 21:33:46
curl        7.88.0       main   2023-02-16 22:01:20
git         2.47.0.2     main   2024-11-10 18:18:50
imagemagick 7.1.2-12     main   2026-01-19 16:49:57
sudo        0.2020.01.26 main   2021-03-29 10:57:13
"
            .lines(),
            ColumnWidthBasis::SeparatorLine,
        )
        .unwrap();
        assert_eq!(column_count, 5);
        assert_eq!(
            cells[0..5],
            ["Name", "Version", "Source", "Updated", "Info"]
        );
        assert_eq!(
            cells[5..10],
            ["7zip", "26.00", "main", "2026-02-25 21:33:46", ""]
        );
        assert_eq!(
            cells[10..15],
            ["curl", "7.88.0", "main", "2023-02-16 22:01:20", ""]
        );
        assert_eq!(
            cells[15..20],
            ["git", "2.47.0.2", "main", "2024-11-10 18:18:50", ""]
        );
        assert_eq!(
            cells[20..25],
            ["imagemagick", "7.1.2-12", "main", "2026-01-19 16:49:57", ""]
        );
        assert_eq!(
            cells[25..30],
            ["sudo", "0.2020.01.26", "main", "2021-03-29 10:57:13", ""]
        );
    }
}
