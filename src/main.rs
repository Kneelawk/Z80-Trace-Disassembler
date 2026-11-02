pub mod queue;
pub mod sparse;

use crate::queue::FixedQueue;
use clap::Parser;
use csv::{StringRecord, Trim};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fmt;
use std::fmt::{Formatter, UpperHex};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Parser)]
struct Args {
    /// Path to a CSV trace file from a logic analyzer of the Z80 in operation
    #[arg(short, long, value_name = "CSV-FILE")]
    pub trace: Vec<PathBuf>,

    /// Path to a CSV file containing information on how to disassemble each instruction
    #[arg(short, long, value_name = "CSV-FILE")]
    pub instructions: PathBuf,
}

#[derive(Debug, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
enum DataType {
    SampleNum,
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
        if upper.contains("SAMPLE") {
            Ok(SampleNum)
        } else if upper.contains("M1") {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CsvInstructionInfo {
    opcode: String,
    mnemonic: String,
    arg_format: String,
    description: String,
}

#[derive(Debug, Clone)]
enum InstructionInfo {
    Opcode {
        opcode: u8,
        mnemonic: String,
        arg_format: String,
        description: String,
    },
    Prefix {
        opcode: u8,
    },
}

#[derive(Debug, Clone)]
struct PrefixedInstructionInfo {
    prefix: u8,
    opcode: u8,
    mnemonic: String,
    arg_format: String,
    description: String,
}

#[derive(Debug, Copy, Clone)]
struct BusElement {
    sample_number: u32,
    address: u16,
    data: u8,
    ty: BusElementType,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum BusElementType {
    Opcode,
    Read,
    Write,
    Output,
}

fn main() {
    let Args {
        trace,
        instructions,
    } = Args::parse();

    println!("Loading instructions...");
    let mut instr_csv = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .comment(Some(b'#'))
        .delimiter(b'|')
        .trim(Trim::All)
        .from_path(&instructions)
        .expect("reader should open instructions file");

    let mut instr_infos = HashMap::<u8, InstructionInfo>::new();
    let mut instr_extra_infos = HashMap::<(u8, u8), PrefixedInstructionInfo>::new();
    for res in instr_csv.deserialize() {
        let csv_info: CsvInstructionInfo = res.expect("instruction csv should deserialize");

        let mut split = csv_info.opcode.split(':');
        let first = split
            .next()
            .expect("instruction csv row should have at least one opcode");
        let first = u8::from_str_radix(first, 16).expect("opcode should be a hex integer");
        if let Some(second) = split.next() {
            let second =
                u8::from_str_radix(second, 16).expect("opcode suffix should be a hex integer");
            instr_infos.insert(first, InstructionInfo::Prefix { opcode: first });
            instr_extra_infos.insert(
                (first, second),
                PrefixedInstructionInfo {
                    prefix: first,
                    opcode: second,
                    mnemonic: csv_info.mnemonic,
                    arg_format: csv_info.arg_format,
                    description: csv_info.description,
                },
            );
        } else {
            instr_infos.insert(
                first,
                InstructionInfo::Opcode {
                    opcode: first,
                    mnemonic: csv_info.mnemonic,
                    arg_format: csv_info.arg_format,
                    description: csv_info.description,
                },
            );
        }
    }

    println!("Instructions loaded.");

    // let mut memory = SparseVec::new();
    let mut memory = BTreeMap::<usize, String>::new();
    for trace in trace {
        println!();
        println!("================================================================");
        println!("Reading: {}", trace.display());
        println!("================================================================");

        let mut csv = csv::ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_path(&trace)
            .expect("reader should open file");

        let mut elements = vec![];
        let mut m1s = FixedQueue::<u16, 3>::new_filled(1);
        let mut mreqs = FixedQueue::<u16, 3>::new_filled(1);
        let mut reads = FixedQueue::<u16, 3>::new_filled(1);
        let mut writes = FixedQueue::<u16, 3>::new_filled(1);
        let mut addresses = FixedQueue::<u16, 3>::new_filled(1);
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
                let sample = get(&header, &record, DataType::SampleNum, 10)
                    .expect("csv should have sample number");
                m1s.push(get(&header, &record, DataType::M1, 16).expect("csv should have M1"));
                mreqs
                    .push(get(&header, &record, DataType::MREQ, 16).expect("csv should have MREQ"));
                reads
                    .push(get(&header, &record, DataType::READ, 16).expect("csv should have READ"));
                writes.push(
                    get(&header, &record, DataType::WRITE, 16).expect("csv should have WRITE"),
                );
                addresses.push(
                    get(&header, &record, DataType::Address, 16).expect("csv should have address"),
                );
                let data = get(&header, &record, DataType::Data, 16).expect("csv should have data");

                if m1s[2] == 1
                    && m1s[1] == 0
                    && mreqs[2] == 1
                    && mreqs[1] == 0
                    && reads[2] == 1
                    && reads[1] == 0
                {
                    elements.push(BusElement {
                        sample_number: sample as u32,
                        address: addresses[1],
                        data: data as u8,
                        ty: BusElementType::Opcode,
                    });
                } else if mreqs[2] == 1 && mreqs[1] == 0 && reads[2] == 1 && reads[1] == 0 {
                    elements.push(BusElement {
                        sample_number: sample as u32,
                        address: addresses[1],
                        data: data as u8,
                        ty: BusElementType::Read,
                    });
                } else if mreqs[2] == 1 && mreqs[1] == 0 && writes[1] == 1 && writes[0] == 0 {
                    elements.push(BusElement {
                        sample_number: sample as u32,
                        address: addresses[1],
                        data: data as u8,
                        ty: BusElementType::Write,
                    });
                } else if reads[1] == 1 && reads[0] == 1 && writes[1] == 1 && writes[0] == 0 {
                    elements.push(BusElement {
                        sample_number: sample as u32,
                        address: addresses[1],
                        data: data as u8,
                        ty: BusElementType::Output,
                    });
                }
            }
        }

        println!("Trace elements:");
        let mut element_iter = elements.iter().enumerate();
        while let Some((index, element)) = element_iter.next() {
            if element.ty == BusElementType::Opcode {
                if let Some(info) = instr_infos.get(&element.data) {
                    match info {
                        InstructionInfo::Opcode {
                            opcode: _,
                            mnemonic,
                            arg_format,
                            description,
                        } => {
                            let args = format_args(&arg_format, &elements[index + 1..]);
                            let instr = format!(
                                "{:<8}{:<16}; @{:04} - {}",
                                mnemonic, args, element.sample_number, description
                            );
                            println!("{:04X}:   {}", element.address, instr);
                            memory.insert(element.address as usize, instr);
                        }
                        InstructionInfo::Prefix { opcode } => {
                            let (index, element_1) = element_iter.next().expect(
                                "prefix element should be followed by another opcode element",
                            );
                            if let Some(PrefixedInstructionInfo {
                                prefix: _,
                                opcode: _,
                                mnemonic,
                                arg_format,
                                description,
                            }) = instr_extra_infos.get(&(*opcode, element_1.data))
                            {
                                let args = format_args(&arg_format, &elements[index + 1..]);
                                let instr = format!(
                                    "{:<8}{:<16}; @{:04} - {}",
                                    mnemonic, args, element.sample_number, description
                                );
                                println!("{:04X}:   {}", element.address, instr);
                                memory.insert(element.address as usize, instr);
                            } else {
                                println!(
                                    "{:04X}:   WARN: unknown partial opcode {:02X}{:02X}",
                                    element.address, element.data, element_1.data
                                );
                                memory.insert(
                                    element.address as usize,
                                    format!("WARN: unknown partial opcode {:02X}{:02X}", element.data, element_1.data),
                                );
                            }
                        }
                    }
                } else {
                    println!(
                        "{:04X}:   WARN: unknown opcode {:02X}",
                        element.address, element.data
                    );
                    memory.insert(
                        element.address as usize,
                        format!("WARN: unknown opcode {:02X}", element.data),
                    );
                }
            } else if element.ty == BusElementType::Write {
                println!("WRITE: {:04X}: {:02X}", element.address, element.data);
            } else if element.ty == BusElementType::Output {
                println!("OUTPUT: {:04X}: {:02X}", element.address, element.data);
            }
        }

        println!("================================================================");
        println!();
    }

    println!("memory: {memory:#04X?}");
}

fn get(
    header: &HashMap<DataType, usize>,
    record: &StringRecord,
    ty: DataType,
    radix: u32,
) -> Option<u16> {
    header
        .get(&ty)
        .and_then(|&idx| record.get(idx))
        .and_then(|elem| u16::from_str_radix(elem.trim_end_matches('h'), radix).ok())
}

fn format_args(format: &str, elements: &[BusElement]) -> String {
    let mut res = String::with_capacity(format.len());
    let mut chars = format.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let c1 = chars.next().expect("% should have an argument");
            if (c1 as u32) >= ('0' as u32) && (c1 as u32) < ('9' as u32) {
                res.push_str(&format!("{:02X}", get_elem(c1, elements).data));
            } else if c1 == 's' {
                let c2 = chars.next().expect("%s should have an argument");
                if (c2 as u32) >= ('0' as u32) && (c2 as u32) < ('9' as u32) {
                    res.push_str(&format!("{:+02X}", ReallySigned(get_elem(c2, elements).data as i8)));
                }
            }
        } else {
            res.push(c);
        }
    }
    res
}

fn get_elem(c1: char, elements: &[BusElement]) -> BusElement {
    let i = (c1 as u32 - '0' as u32) as usize;
    let elem = elements[i];
    if elem.ty != BusElementType::Read {
        panic!(
            "attempting to parse argument {} but it is not a read: {:?}",
            i, elem
        );
    }
    elem
}

// Copied from https://stackoverflow.com/a/44712309
struct ReallySigned(i8);

impl UpperHex for ReallySigned {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let prefix = if f.alternate() { "0x" } else { "" };
        let bare_hex = format!("{:X}", self.0.abs());
        f.pad_integral(self.0 >= 0, prefix, &bare_hex)
    }
}
