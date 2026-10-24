//! Attribute data types and their SQL spelling per DBMS.
//!
//! Decoded from the `CUnParametre` fields used by the type formatter at
//! 0x1400a7f70 (`param_4` is the DBMS index).

use loo_format::Parametre;
use serde::Serialize;

/// Target DBMS, numbered as in Looping (`CLoopingDoc+0x52c`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum Dbms {
    Access = 0,
    MySql = 1,
    Oracle = 2,
    PostgreSql = 3,
    SqlServer = 4,
    Sybase = 5,
    Db2 = 6,
    MariaDb = 7,
    Sqlite = 8,
}

impl Dbms {
    pub const ALL: [Dbms; 9] = [
        Dbms::Access,
        Dbms::MariaDb,
        Dbms::MySql,
        Dbms::Oracle,
        Dbms::PostgreSql,
        Dbms::SqlServer,
        Dbms::Sqlite,
        Dbms::Sybase,
        Dbms::Db2,
    ];

    pub fn from_index(i: i32) -> Dbms {
        match i {
            1 => Dbms::MySql,
            2 => Dbms::Oracle,
            3 => Dbms::PostgreSql,
            4 => Dbms::SqlServer,
            5 => Dbms::Sybase,
            6 => Dbms::Db2,
            7 => Dbms::MariaDb,
            8 => Dbms::Sqlite,
            _ => Dbms::Access,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Dbms::Access => "Access",
            Dbms::MySql => "MySQL",
            Dbms::Oracle => "Oracle",
            Dbms::PostgreSql => "PostgreSQL",
            Dbms::SqlServer => "SQL Server",
            Dbms::Sybase => "Sybase",
            Dbms::Db2 => "DB2",
            Dbms::MariaDb => "MariaDB",
            Dbms::Sqlite => "SQLite",
        }
    }

    fn col(self) -> usize {
        self as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Charset {
    Ascii,
    Unicode,
    Binary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TextKind {
    Fixed,
    Variable,
    Large,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum DataType {
    Text {
        charset: Charset,
        kind: TextKind,
        length: i32,
        collation: String,
    },
    /// bits: 8, 16, 32 or 64
    Integer {
        bits: u8,
    },
    Decimal {
        precision: i32,
        scale: i32,
    },
    Real {
        double: bool,
    },
    Currency,
    Counter,
    Date,
    Time,
    DateTime {
        timezone: bool,
    },
    Boolean,
    Xml,
    Geometry,
    Geography,
    /// Free-form type text (category 4, `CUnParametre+0x40`) as typed: the
    /// type, then optionally `#` and a description, see [`DataType::free`].
    Free(String),
    Unknown,
}

impl DataType {
    pub fn from_param(p: &Parametre) -> DataType {
        let [num_kind, int_size, real_kind, date_kind] = p.f2c;
        match p.f20 {
            0 => {
                let charset = match p.f28 {
                    1 => Charset::Unicode,
                    2 => Charset::Binary,
                    _ => Charset::Ascii,
                };
                let kind = match p.f24 {
                    0 => TextKind::Fixed,
                    2 => TextKind::Large,
                    _ => TextKind::Variable,
                };
                // 0x64..0x78 hold one length per (charset, kind) pair
                let [nchar, binary, varchar, nvarchar, varbinary] = p.f68;
                let length = match (charset, kind) {
                    (Charset::Ascii, TextKind::Fixed) => p.f64,
                    (Charset::Unicode, TextKind::Fixed) => nchar,
                    (Charset::Binary, TextKind::Fixed) => binary,
                    (Charset::Ascii, _) => varchar,
                    (Charset::Unicode, _) => nvarchar,
                    (Charset::Binary, _) => varbinary,
                };
                let collation = match kind {
                    TextKind::Variable => &p.s48,
                    TextKind::Fixed => &p.s50,
                    TextKind::Large => &p.s58,
                }
                .clone();
                DataType::Text { charset, kind, length, collation }
            }
            1 => match num_kind {
                0 => DataType::Integer {
                    bits: match int_size {
                        0 => 8,
                        1 => 16,
                        3 => 64,
                        _ => 32,
                    },
                },
                1 => DataType::Decimal { precision: p.f7c, scale: p.f80 },
                2 => DataType::Real { double: real_kind != 0 },
                3 => DataType::Currency,
                4 => DataType::Counter,
                _ => DataType::Unknown,
            },
            2 => match date_kind {
                0 => DataType::Date,
                1 => DataType::Time,
                _ => DataType::DateTime { timezone: p.f60 != 0 },
            },
            3 => DataType::Boolean,
            // `CUnParametre+0x40` (0x1400a7f70): the text before a `#`, the
            // rest being a description
            4 => DataType::Free(p.s40.clone()),
            5 => DataType::Xml,
            // radio buttons 14 « Géographique » / 13 « Géométrique » of the
            // « Rubrique » dialog (0x1400386c0)
            6 => DataType::Geography,
            7 => DataType::Geometry,
            _ => DataType::Unknown,
        }
    }

    /// SQL type for `dbms`. `as_foreign_key` turns counters into plain
    /// integers, as Looping does for migrated keys.
    /// A free type (0x1400a7f70): the text before `#`; a foreign key
    /// column gets the description after it instead (r018).
    fn free(s: &str, as_foreign_key: bool) -> String {
        match s.split_once('#') {
            Some((_, description)) if as_foreign_key => description.trim().to_string(),
            Some((name, _)) => name.trim_end().to_string(),
            None => s.to_string(),
        }
    }

    pub fn sql(&self, dbms: Dbms, as_foreign_key: bool) -> String {
        let c = dbms.col();
        // Columns: Access, MySQL, Oracle, PostgreSQL, SQL Server, Sybase, DB2, MariaDB, SQLite
        let pick = |t: [&str; 9]| t[c].to_string();
        match self {
            DataType::Text { charset, kind, length, collation } => {
                let base = match (charset, kind) {
                    (Charset::Ascii, TextKind::Variable) => {
                        if dbms == Dbms::Sqlite {
                            "TEXT".into()
                        } else {
                            let f = [
                                "VARCHAR(",
                                "VARCHAR(",
                                "VARCHAR2(",
                                "VARCHAR(",
                                "VARCHAR(",
                                "VARCHAR(",
                                "VARCHAR(",
                                "VARCHAR(",
                                "",
                            ][c];
                            format!("{f}{length})")
                        }
                    }
                    (Charset::Unicode, TextKind::Variable) => {
                        if dbms == Dbms::Sqlite {
                            "TEXT".into()
                        } else {
                            let f = [
                                "VARCHAR(",
                                "VARCHAR(",
                                "NVARCHAR2(",
                                "VARCHAR(",
                                "NVARCHAR(",
                                "NVARCHAR(",
                                "VARGRAPHIC(",
                                "VARCHAR(",
                                "",
                            ][c];
                            format!("{f}{length})")
                        }
                    }
                    (Charset::Binary, TextKind::Variable) => {
                        if dbms == Dbms::Sqlite {
                            "TEXT".into()
                        } else {
                            let f = [
                                "VARBINARY(",
                                "VARBINARY(",
                                "VARBINARY(",
                                "BIT VARYING(",
                                "VARBINARY(",
                                "VARBINARY(",
                                "VARBINARY(",
                                "VARBINARY(",
                                "",
                            ][c];
                            format!("{f}{length})")
                        }
                    }
                    (Charset::Ascii, TextKind::Fixed) => {
                        if dbms == Dbms::Sqlite {
                            "TEXT".into()
                        } else {
                            format!("CHAR({length})")
                        }
                    }
                    (Charset::Unicode, TextKind::Fixed) => {
                        if dbms == Dbms::Sqlite {
                            "TEXT".into()
                        } else {
                            let f =
                                ["CHAR(", "CHAR(", "NCHAR(", "CHAR(", "NCHAR(", "NCHAR(", "GRAPHIC(", "CHAR(", ""][c];
                            format!("{f}{length})")
                        }
                    }
                    (Charset::Binary, TextKind::Fixed) => {
                        if dbms == Dbms::Sqlite {
                            "TEXT".into()
                        } else {
                            let f = [
                                "BINARY(", "BINARY(", "BINARY(", "BIT(", "BINARY(", "BINARY(", "BINARY(", "BINARY(", "",
                            ][c];
                            format!("{f}{length})")
                        }
                    }
                    (Charset::Ascii, TextKind::Large) => {
                        pick(["TEXT", "TEXT", "CLOB", "TEXT", "VARCHAR(max)", "TEXT", "CLOB", "TEXT", "TEXT"])
                    }
                    (Charset::Unicode, TextKind::Large) => {
                        pick(["TEXT", "TEXT", "NCLOB", "TEXT", "NVARCHAR(max)", "UNITEXT", "DBCLOB", "TEXT", "TEXT"])
                    }
                    (Charset::Binary, TextKind::Large) => {
                        pick(["LONGBINARY", "BLOB", "BLOB", "BYTEA", "VARBINARY(max)", "IMAGE", "BLOB", "BLOB", "BLOB"])
                    }
                };
                if collation.is_empty() {
                    base
                } else {
                    format!("{base} COLLATE {collation}")
                }
            }
            DataType::Integer { bits } => match bits {
                8 => pick([
                    "BYTE",
                    "TINYINT",
                    "NUMBER(3)",
                    "SMALLINT",
                    "TINYINT",
                    "TINYINT",
                    "SMALLINT",
                    "TINYINT",
                    "INTEGER",
                ]),
                16 => pick([
                    "SMALLINT",
                    "SMALLINT",
                    "NUMBER(5)",
                    "SMALLINT",
                    "SMALLINT",
                    "SMALLINT",
                    "SMALLINT",
                    "SMALLINT",
                    "INTEGER",
                ]),
                64 => pick([
                    "BIGINT",
                    "BIGINT",
                    "NUMBER(20)",
                    "BIGINT",
                    "BIGINT",
                    "BIGINT",
                    "BIGINT",
                    "BIGINT",
                    "INTEGER",
                ]),
                _ => pick(["INT", "INT", "NUMBER(10)", "INTEGER", "INT", "INT", "INT", "INT", "INTEGER"]),
            },
            DataType::Decimal { precision, scale } => {
                let f = match dbms {
                    Dbms::Oracle => "NUMBER(",
                    Dbms::PostgreSql | Dbms::Sqlite => "NUMERIC(",
                    _ => "DECIMAL(",
                };
                format!("{f}{precision},{scale})")
            }
            DataType::Real { double } => {
                if *double {
                    pick([
                        "DOUBLE",
                        "DOUBLE",
                        "BINARY_DOUBLE",
                        "DOUBLE PRECISION",
                        "FLOAT",
                        "DOUBLE PRECISION",
                        "DOUBLE",
                        "DOUBLE",
                        "REAL",
                    ])
                } else {
                    pick(["REAL", "FLOAT", "BINARY_FLOAT", "REAL", "REAL", "REAL", "REAL", "FLOAT", "REAL"])
                }
            }
            DataType::Currency => pick([
                "CURRENCY",
                "DECIMAL(19,4)",
                "NUMBER(19,4)",
                "MONEY",
                "MONEY",
                "MONEY",
                "DECIMAL(19,4)",
                "DECIMAL(19,4)",
                "NUMERIC",
            ]),
            DataType::Counter => {
                if as_foreign_key {
                    pick(["INT", "INT", "NUMBER(10)", "INTEGER", "INT", "INT", "INT", "INT", "INTEGER"])
                } else {
                    pick([
                        "COUNTER",
                        "INT AUTO_INCREMENT",
                        "NUMBER(10)",
                        "SERIAL",
                        "INT IDENTITY",
                        "INT IDENTITY",
                        "INT GENERATED BY DEFAULT AS IDENTITY",
                        "INT AUTO_INCREMENT",
                        "INTEGER",
                    ])
                }
            }
            DataType::Date => pick(["DATE", "DATE", "DATE", "DATE", "DATE", "DATE", "DATE", "DATE", "NUMERIC"]),
            DataType::Time => pick(["TIME", "TIME", "DATE", "TIME", "TIME", "TIME", "TIME", "TIME", "NUMERIC"]),
            DataType::DateTime { timezone: false } => pick([
                "DATETIME",
                "DATETIME",
                "TIMESTAMP",
                "TIMESTAMP",
                "DATETIME2",
                "BIGDATETIME",
                "TIMESTAMP",
                "DATETIME",
                "NUMERIC",
            ]),
            DataType::DateTime { timezone: true } => pick([
                "DATETIME",
                "DATETIME",
                "TIMESTAMP WITH TIME ZONE",
                "TIMESTAMPZ",
                "DATETIMEOFFSET",
                "TIMESTAMP WITH TIME ZONE",
                "TIMESTAMP",
                "DATETIME",
                "NUMERIC",
            ]),
            DataType::Boolean => {
                pick(["LOGICAL", "BOOLEAN", "NUMBER(1)", "BOOLEAN", "BIT", "BIT", "SMALLINT", "BOOLEAN", "NUMERIC"])
            }
            DataType::Xml => pick(["TEXT", "TEXT", "XMLTYPE", "XML", "XML", "XML", "XML", "TEXT", "TEXT"]),
            DataType::Geometry => pick([
                "TEXT",
                "GEOMETRY",
                "SDO_GEOMETRY",
                "GEOMETRY",
                "GEOMETRY",
                "ST_GEOMETRY",
                "ST_GEOMETRY",
                "GEOMETRY",
                "TEXT",
            ]),
            DataType::Geography => pick([
                "TEXT",
                "GEOMETRY",
                "SDO_GEOMETRY",
                "GEOGRAPHY",
                "GEOGRAPHY",
                "ST_GEOMETRY",
                "ST_GEOMETRY",
                "GEOMETRY",
                "TEXT",
            ]),
            DataType::Free(s) => DataType::free(s, as_foreign_key),
            DataType::Unknown => String::new(),
        }
    }

    /// Short label used in the conceptual view (Looping's "Type données" column).
    pub fn label(&self) -> String {
        match self {
            DataType::Text { kind: TextKind::Large, .. } => "Texte long".into(),
            DataType::Text { length, .. } => format!("Texte({length})"),
            DataType::Integer { .. } => "Entier".into(),
            DataType::Decimal { precision, scale } => format!("Décimal({precision},{scale})"),
            DataType::Real { .. } => "Réel".into(),
            DataType::Currency => "Monétaire".into(),
            DataType::Counter => "Compteur".into(),
            DataType::Date => "Date".into(),
            DataType::Time => "Heure".into(),
            DataType::DateTime { .. } => "Date-Heure".into(),
            DataType::Boolean => "Booléen".into(),
            DataType::Xml => "XML".into(),
            DataType::Geometry => "Géométrique".into(),
            DataType::Geography => "Géographique".into(),
            DataType::Free(s) => DataType::free(s, false),
            DataType::Unknown => String::new(),
        }
    }
}
