pub mod sparse;

use crate::sparse::SparseVec;
use clap::Parser;
use csv::StringRecord;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Parser)]
struct Args {
    /// Path to a CSV trace file from a logic analyzer of the Z80 in operation
    #[arg(short, long, value_name = "FILE")]
    pub trace: Vec<PathBuf>,
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
enum DataType {
    M1,
    MREQ,
    READ,
    WRITE,
    CSROM,
    CSRAM,
    Data,
    Address,
}

impl FromStr for DataType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        use DataType::*;

        let upper = s.to_uppercase();
        if upper.contains("M1") {
            Ok(M1)
        } else if upper.contains("MREQ") {
            Ok(MREQ)
        } else if upper.contains("READ") {
            Ok(READ)
        } else if upper.contains("WRITE") {
            Ok(WRITE)
        } else if upper.contains("CSROM") {
            Ok(CSROM)
        } else if upper.contains("CSRAM") {
            Ok(CSRAM)
        } else if upper.contains("DATA") {
            Ok(Data)
        } else if upper.contains("ADDRESS") {
            Ok(Address)
        } else {
            Err(())
        }
    }
}

fn main() {
    let Args { trace } = Args::parse();

    let mut memory = SparseVec::new();
    for trace in trace {
        let mut csv = csv::ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_path(&trace)
            .expect("reader should open file");

        let mut header = HashMap::new();
        for record in csv.records() {
            let record = record.expect("record should record");
            // ignore the first part
            if record.len() < 6 {
                continue;
            }

            if header.is_empty() {
                for (index, element) in record.iter().enumerate() {
                    let Ok(ty) = DataType::from_str(element) else {
                        continue;
                    };

                    header.insert(ty, index);
                }
            } else {
                let m1 = get(&header, &record, DataType::M1).expect("csv should have M1");
                let mreq = get(&header, &record, DataType::MREQ).expect("csv should have MREQ");
                let data = get(&header, &record, DataType::Data).expect("csv should have Data");
                let address =
                    get(&header, &record, DataType::Address).expect("csv should have address");

                if m1 == 0 && mreq == 0 {
                    memory.set(address as usize, data as u8);
                }
            }
        }
    }

    println!("memory: {memory:#X?}");
}

fn get(header: &HashMap<DataType, usize>, record: &StringRecord, ty: DataType) -> Option<u16> {
    header
        .get(&ty)
        .and_then(|&idx| record.get(idx))
        .and_then(|elem| u16::from_str_radix(elem.trim_end_matches('h'), 16).ok())
}
