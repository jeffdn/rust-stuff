extern crate postgres;
extern crate chrono;

#[cfg(test)]
mod tests;

use postgres::{Connection, SslMode, types};
use std::io::{self, Read};

#[derive(PartialEq, Debug)]
pub enum Align {
    Left,
    Right,
    Center,
}

#[allow(dead_code)]
#[derive(PartialEq, Debug)]
pub enum Color {
    White,
    BoldWhite,
    BoldRed,
    BoldGreen,
    BoldBlue,
}

struct TableDesc {
    names:  Vec<String>,
    types:  Vec<types::Type>,
    widths: Vec<usize>,
    data:   Vec<Vec<String>>,
}


impl TableDesc {
    fn new() -> TableDesc {
        TableDesc {
            names:  Vec::new(),
            types:  Vec::new(),
            widths: Vec::new(),
            data:   Vec::new(),
        }
    }

    fn register_column(&mut self, column: &postgres::stmt::Column) {
        let colname = String::from(column.name());

        self.widths.push(colname.len());
        self.names.push(colname);
        self.types.push(column.type_().clone());
    }

    fn parse_result(&self, column: &postgres::rows::Row, colpos: usize) -> String {
        let val: String = match self.types[colpos] {
            types::Type::Text | types::Type::Varchar => column.get(colpos),
            types::Type::Bool => {
                let tmpbool: bool = column.get(colpos);
                match tmpbool {
                    true  => String::from("true"),
                    false => String::from("false"),
                }
            },
            types::Type::Int2 | types::Type::Int4 => {
                let tmpint: i32 = column.get(colpos);
                tmpint.to_string()
            },
            types::Type::Int8 => {
                let tmpint: i64 = column.get(colpos);
                tmpint.to_string()
            },
            types::Type::Float4 => {
                let tmpflt: f32 = column.get(colpos);
                tmpflt.to_string()
            },
            types::Type::Float8 => {
                let tmpflt: f64 = column.get(colpos);
                tmpflt.to_string()
            },
            types::Type::Timestamp => {
                let tmpdate: chrono::NaiveDateTime = column.get(colpos);
                tmpdate.format("%Y-%m-%d %H:%M:%S").to_string()
            },
            types::Type::TimestampTZ => {
                let tmpdate: chrono::DateTime<chrono::UTC> = column.get(colpos);
                tmpdate.format("%Y-%m-%d %H:%M:%s").to_string()
            },
            types::Type::Date => {
                let tmpdate: chrono::NaiveDate = column.get(colpos);
                tmpdate.format("%Y-%m-%d").to_string()
            },
            _ => String::from(""),
        };

        val
    }

    fn append(&mut self, row: &postgres::rows::Row) {
        let mut colvals: Vec<String> = Vec::new();

        for i in 0..row.len() {
            let ret = row.get_bytes(i);
            let col = match ret.is_some() {
                true    => self.parse_result(&row, i),
                false   => String::from(""),
            };

            self.widths[i] = match col.len() > self.widths[i] {
                true    => col.len(),
                false   => self.widths[i]
            };

            colvals.push(col);
        }

        self.data.push(colvals);
    }

    fn print_row(&self, rowdata: &Vec<String>, is_header: bool) {
        for (i, col) in rowdata.iter().enumerate() {
            if i > 0 {
                print!("{}", TableDesc::color_text("|", Color::BoldWhite));
            }

            match is_header {
                false   => print!(" {} ", TableDesc::format_field(&col,
                                                            self.widths[i],
                                                            TableDesc::get_alignment(&self.types[i]))),
                true    => print!(" {} ", TableDesc::color_text(&TableDesc::format_field(&col,
                                                                             self.widths[i],
                                                                             Align::Center),
                                                          Color::BoldWhite)),
            };
        }

        println!("");

        match is_header {
            false   => {},
            true    => {
                for (i, col) in self.widths.iter().enumerate() {
                    if i > 0 {
                        print!("{}", TableDesc::color_text("+", Color::BoldWhite));
                    }

                    print!("{}", TableDesc::color_text(&TableDesc::pad_gen(col+2, "-"), Color::BoldWhite));
                }

                println!("");
            },
        };

    }

    fn print(&self) {
        self.print_row(&self.names, true);
        for rowdata in &self.data {
            self.print_row(&rowdata, false);
        }
    }
}

/* public functions, for string formatting, etc. */
impl TableDesc {
    pub fn get_alignment(coltype: &types::Type) -> Align {
        match coltype {
            &types::Type::Int2          => Align::Right,
            &types::Type::Int4          => Align::Right,
            &types::Type::Int8          => Align::Right,
            &types::Type::Float4        => Align::Right,
            &types::Type::Float8        => Align::Right,
            &types::Type::Date          => Align::Right,
            &types::Type::Timestamp     => Align::Right,
            &types::Type::TimestampTZ   => Align::Right,
            _                           => Align::Left,
        }
    }

    pub fn pad_gen(len: usize, pad: &str) -> String {
        let mut padstr = String::from("");

        for _ in 0..len {
            padstr = padstr + pad;
        }

        padstr
    }

    pub fn format_field(column: &str, width: usize, align: Align) -> String {
        let padlen: usize = width - column.len();
        let extra: usize = padlen % 2;

        let ret: String = match align {
            Align::Right    => { format!("{}{}", TableDesc::pad_gen(padlen, " "), column) },
            Align::Left     => { format!("{}{}", column, TableDesc::pad_gen(padlen, " ")) },
            Align::Center   => { format!("{}{}{}", TableDesc::pad_gen(padlen/2, " "), column,
                                                   TableDesc::pad_gen((padlen/2)+extra, " ")) },
        };

        ret
    }

    pub fn color_text(text: &str, color: Color) -> String {
        let clrstr = match color {
            Color::White       => { format!("{}{}\x1B[33m\x1B[0m", "\x1B[33m\x1B[37m", text)        },
            Color::BoldWhite   => { format!("{}{}\x1B[33m\x1B[0m", "\x1B[33m\x1B[1m\x1B[33m\x1B[37m", text) },
            Color::BoldRed     => { format!("{}{}\x1B[33m\x1B[0m", "\x1B[33m\x1B[1m\x1B[33m\x1B[31m", text) },
            Color::BoldGreen   => { format!("{}{}\x1B[33m\x1B[0m", "\x1B[33m\x1B[1m\x1B[33m\x1B[32m", text) },
            Color::BoldBlue    => { format!("{}{}\x1B[33m\x1B[0m", "\x1B[33m\x1B[1m\x1B[33m\x1B[34m", text) },
        };

        clrstr
    }
}

fn main() {
    let mut buf = String::new();
    let mut tdesc = TableDesc::new();
    let mut args = std::env::args();

    if args.len() == 1 {
        println!("{} {}", TableDesc::color_text("error:", Color::BoldRed),
                          TableDesc::color_text("a DBURI is required!", Color::BoldWhite));
        println!("{} cat file.sql | first [DBURI]", TableDesc::color_text("usage:", Color::BoldBlue));
        std::process::exit(1);
    }

    let connstr: String = args.nth(1).unwrap();
    io::stdin().read_to_string(&mut buf);

    let conn = Connection::connect(&connstr as &str, SslMode::None).unwrap();
    let res = &conn.query(&mut buf, &[]).unwrap();

    for col in res.columns() {
        tdesc.register_column(&col);
    }

    for row in res.iter() {
        tdesc.append(&row);
    }

    tdesc.print();
}
