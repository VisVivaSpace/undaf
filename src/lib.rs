#![allow(unused)] // TODO remove later

use crate::prelude::*;

mod error;
mod prelude;

#[cfg(target_endian = "big")]
pub const NATIVE_ENDIAN: Endian = Endian::Big;

#[cfg(target_endian = "little")]
pub const NATIVE_ENDIAN: Endian = Endian::Little;

fn get_f64(mut f: &File, offset: u64, endian: &Endian) -> Result<f64> {
    f.seek(SeekFrom::Start(offset))?;

    let mut buf: [u8; 8] = [0; 8];
    f.read_exact(&mut buf)?;

    match endian {
        Endian::Little => Ok(f64::from_le_bytes(buf)),
        Endian::Big => Ok(f64::from_be_bytes(buf)),
    }
}

fn get_f64vec(f: &File, offset1: u64, offset2: u64, endian: &Endian) -> Result<Vec<f64>> {
    let vec_size = (offset2 - offset1) * 4;
    let mut vectr = Vec::with_capacity(vec_size as usize);

    for offset in (offset1..offset2).step_by(4) {
        vectr.push(get_f64(f, offset, endian)?);
    }

    Ok(vectr)
}

fn get_char(mut f: &File, offset: u64) -> Result<char> {
    f.seek(SeekFrom::Start(offset))?;

    let mut buf: [u8; 1] = [0];
    f.read_exact(&mut buf)?;

    if buf[0].is_ascii() {
        return Ok(buf[0] as char);
    } else {
        return Err(anyhow!("Non-ASCII value read"));
    };
}

fn get_i32(mut f: &File, offset: u64, endian: &Endian) -> Result<i32> {
    f.seek(SeekFrom::Start(offset))?;

    let mut buf: [u8; 4] = [0; 4];
    f.read_exact(&mut buf)?;

    match endian {
        Endian::Little => Ok(i32::from_le_bytes(buf)),
        Endian::Big => Ok(i32::from_be_bytes(buf)),
    }
}

fn get_string(mut f: &File, offset: u64, maxlen: u64) -> Result<String> {
    f.seek(SeekFrom::Start(offset))?;
    let mut byte = f.bytes();
    let mut string_out = String::with_capacity(maxlen as usize);

    for _ in 1..maxlen {
        let b = byte.next();

        match b {
            None => {
                break; // if b none, end of string
            }
            Some(Err(error)) => {
                return Err(anyhow!(error));
            }
            Some(Ok(0u8)) => {
                // if \0, skip
                continue;
            }
            Some(Ok(4u8)) => {
                // end of transmission ascii char
                break;
            }
            Some(Ok(c)) => {
                if c.is_ascii() {
                    // add to string and repeat
                    string_out.push(c as char);
                } else {
                    // if not ascii, skip
                    continue;
                }
            }
        };
    }
    Ok(string_out.trim().to_string())
}

#[derive(Debug, Serialize)]
pub enum Endian {
    Big,
    Little,
}

impl Endian {
    fn from_daf_file(f: &File) -> Result<Endian> {
        match get_char(f, 88)? {
            'B' | 'b' => Ok(Endian::Big),
            'L' | 'l' => Ok(Endian::Little),
            _ => Err(anyhow!("Unable to determine DAF file endian-ness")),
        }
    }
}

/*

# DAF File Record Structure

From: [https://naif.jpl.nasa.gov/pub/naif/toolkit_docs/C/req/daf.html#The%20File%20Record]

The file record is always the first physical record in a DAF. The record size is 1024 bytes (for platforms with one byte char size, and four bytes integer size). The items listed in the File Record:

1. LOCIDW (8 characters, 8 bytes): An identification word (`DAF/xxxx').
  The 'xxxx' substring is a string of four characters or less indicating the type of data stored in the DAF file. This is used by the SPICELIB subroutines to verify that a particular file is in fact a DAF and not merely a direct access file with the same record length. When a DAF is opened, an error signals if this keyword is not present. [Address 0]
2. ND ( 1 integer, 4 bytes): The number of double precision components in each array summary. [Address 8]
3. NI ( 1 integer, 4 bytes): The number of integer components in each array summary. [Address 12]
4. LOCIFN (60 characters, 60 bytes): The internal name or description of the array file. [Address 16]
5. FWARD ( 1 integer, 4 bytes): The record number of the initial summary record in the file. [Address 76]
6. BWARD ( 1 integer, 4 bytes): The record number of the final summary record in the file. [Address 80]
7. FREE ( 1 integer, 4 bytes): The first free address in the file. This is the address at which the first element of the next array to be added to the file will be stored. [Address 84]
8. LOCFMT ( 8 characters, 8 bytes): The character string that indicates the numeric binary format of the DAF. The string has value either "LTL-IEEE" or "BIG-IEEE." [Address 88]
9. PRENUL ( 603 characters, 603 bytes): A block of nulls to pad between the last character of LOCFMT and the first character of FTPSTR to keep FTPSTR at character 700 (address 699) in a 1024 byte record. [Address 96]
10. FTPSTR ( 28 characters, 28 bytes): The FTP validation string.
  This string is assembled using components returned from the SPICELIB private routine ZZFTPSTR. [Address 699]
11. PSTNUL ( 297 characters, 297 bytes): A block of nulls to pad from the last character of FTPSTR to the end of the file record. Note: this value enforces the length of the file record as 1024 bytes. [Address 727]

*/

type SegReader = fn(&mut DAFFile, u64) -> Result<DAFSegment>;

#[derive(Debug)]
pub struct DAFFile {
    file: File,
    pub endian: Endian,
    daf_type: char,
    seg_reader: SegReader,
    nd: u64,
    ni: u64,
    locifn: String,
    fward: u64,
    bward: u64,
    free_address: u64,
    pub ftpstr: String,
    current_record: u64,
    namerec_offset: u64,
    next_record: u64,
    sum_size: u64,
    nc: u64,
    current_segment: u64,
    nsum: u64,
}

impl DAFFile {
    pub fn from_file(file: File) -> Result<DAFFile> {
        let endian = Endian::from_daf_file(&file)?;
        let daf_type = get_char(&file, 4)?;
        let nd = get_i32(&file, 8, &endian)? as u64;
        let ni = get_i32(&file, 12, &endian)? as u64;
        let locifn = get_string(&file, 16, 60)?;
        let fward = get_i32(&file, 76, &endian)? as u64;
        let bward = get_i32(&file, 80, &endian)? as u64;
        let free_address = get_i32(&file, 84, &endian)? as u64;
        let ftpstr = get_string(&file, 699, 28)?;
        let current_record = fward as u64;

        let namerec_offset = 1024 * (bward - fward + 1) as u64;
        let sum_size = (8 * nd + 4 * ni) as u64;
        let nc = 8 * (nd + (ni + 1) / 2) as u64;

        let next_record = get_f64(&file, 1024 * (current_record - 1), &endian)? as u64;
        let nsum = get_f64(&file, 1024 * (current_record - 1) + 16, &endian)? as u64;

        let current_segment: u64 = 0;

        let seg_reader = match daf_type {
            'S' => SPKSegment::reader,
            'C' => CKSegment::reader,
            'P' => BPCKSegment::reader,
            _ => {
                return Err(anyhow!("Unsuported DAF file type"));
            }
        };

        Ok(DAFFile {
            file,
            endian,
            daf_type,
            seg_reader,
            nd,
            ni,
            locifn,
            fward,
            bward,
            free_address,
            ftpstr,
            current_record,
            namerec_offset,
            next_record,
            sum_size,
            nc,
            current_segment,
            nsum,
        })
    }

    pub fn read_f64(&mut self, offset: u64) -> Result<f64> {
        get_f64(&self.file, offset, &self.endian)
    }

    pub fn read_f64vec(&mut self, offset1: u64, offset2: u64) -> Result<Vec<f64>> {
        get_f64vec(&self.file, offset1, offset2, &self.endian)
    }

    pub fn read_char(&mut self, offset: u64) -> Result<char> {
        get_char(&self.file, offset)
    }

    pub fn read_i32(&mut self, offset: u64) -> Result<i32> {
        get_i32(&self.file, offset, &self.endian)
    }

    pub fn read_string(&mut self, offset: u64, maxlen: u64) -> Result<String> {
        get_string(&self.file, offset, maxlen)
    }

    pub fn comment(&mut self) -> Result<String> {
        if self.fward > 2 {
            let offset: u64 = 1024; // DAF comments start at record 2 (address 1024)
            let maxlen: u64 = 1024 * (self.fward - 1); // DAF comments end at the summary record
            let comment = self.read_string(offset, maxlen)?;

            return Ok(comment);
        }

        // if summaries start at record 2 there are no comments;
        Ok("".to_string())
    }

    pub fn daf_header(&mut self) -> Result<DAFHeader> {
        Ok(DAFHeader {
            name: self.locifn.clone(),
            comment: self.comment()?,
            kind: match self.daf_type {
                'S' => "SPK".to_string(),
                'C' => "CK".to_string(),
                'P' => "BPCK".to_string(),
                _ => "unknown".to_string(),
            },
        })
    }

    pub fn segment_reader(&mut self, offset: u64) -> Result<DAFSegment> {
        (self.seg_reader)(self, offset)
    }

    pub fn current_ptr(&mut self) -> u64 {
        1024 * (self.current_record - 1) + 24 + self.current_segment * self.sum_size
    }

    fn advance_record(&mut self) -> Option<Result<u64>> {
        if self.next_record == 0 {
            return None;
        } else {
            let offset = (self.next_record - 1) * 1024;
            let new_next = match self.read_f64(offset) {
                Ok(s) => s as u64,
                Err(e) => return Some(Err(e)),
            };
            let new_nsum = match self.read_f64(offset + 16) {
                Ok(s) => s as u64,
                Err(e) => return Some(Err(e)),
            };
            self.current_record = self.next_record;
            self.next_record = new_next;
            self.nsum = new_nsum;
            self.current_segment = 0;
            return Some(Ok(self.current_ptr()));
        }
    }

    fn advance_segment(&mut self) -> Option<Result<u64>> {
        if self.current_segment < self.nsum {
            self.current_segment = self.current_segment + 1;
            return Some(Ok(self.current_ptr()));
        } else {
            return self.advance_record();
        }
    }
}

impl Iterator for DAFFile {
    type Item = Result<DAFSegment>;
    //change to return DAFSegment ...

    fn next(&mut self) -> Option<Self::Item> {
        match self.advance_segment() {
            Some(Ok(s)) => Some(self.segment_reader(s)),
            Some(Err(e)) => Some(Err(e)),
            None => None,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct SPK1 {
    epochs: Vec<f64>,
    records: Vec<Vec<f64>>,
}

#[derive(Debug, Serialize)]
pub struct SPK2 {
    init_epoch: f64,
    tstep: f64,
    midpoints: Vec<f64>,
    radii: Vec<f64>,
    rx_coefficients: Vec<Vec<f64>>,
    ry_coefficients: Vec<Vec<f64>>,
    rz_coefficients: Vec<Vec<f64>>,
    degree: u32,
}

#[derive(Debug, Serialize)]
pub struct SPK3 {
    init_epoch: f64,
    tstep: f64,
    midpoints: Vec<f64>,
    radii: Vec<f64>,
    rx_coefficients: Vec<Vec<f64>>,
    ry_coefficients: Vec<Vec<f64>>,
    rz_coefficients: Vec<Vec<f64>>,
    vx_coefficients: Vec<Vec<f64>>,
    vy_coefficients: Vec<Vec<f64>>,
    vz_coefficients: Vec<Vec<f64>>,
    degree: u32,
}

#[derive(Debug, Serialize)]
pub struct SPK5 {
    gm: f64,
    epochs: Vec<f64>,
    states: Vec<Vec<f64>>,
}

#[derive(Debug, Serialize)]
pub struct SPK8 {
    init_epoch: f64,
    tstep: f64,
    rx_coefficients: Vec<Vec<f64>>,
    ry_coefficients: Vec<Vec<f64>>,
    rz_coefficients: Vec<Vec<f64>>,
    vx_coefficients: Vec<Vec<f64>>,
    vy_coefficients: Vec<Vec<f64>>,
    vz_coefficients: Vec<Vec<f64>>,
    degree: u32,
}

#[derive(Debug, Serialize)]
pub struct SPK9 {
    epochs: Vec<f64>,
    rx_coefficients: Vec<Vec<f64>>,
    ry_coefficients: Vec<Vec<f64>>,
    rz_coefficients: Vec<Vec<f64>>,
    vx_coefficients: Vec<Vec<f64>>,
    vy_coefficients: Vec<Vec<f64>>,
    vz_coefficients: Vec<Vec<f64>>,
    degree: u32,
}

#[derive(Debug, Serialize)]
pub struct SPK10 {
    data: Vec<f64>,
}

#[derive(Debug, Serialize)]
pub struct SPK12 {
    data: Vec<f64>,
}

#[derive(Debug, Serialize)]
pub struct SPK13 {
    data: Vec<f64>,
}

#[derive(Debug, Serialize)]
pub struct SPK14 {
    data: Vec<f64>,
}

#[derive(Debug, Serialize)]
pub struct SPK15 {
    data: Vec<f64>,
}

#[derive(Debug, Serialize)]
pub struct SPK17 {
    data: Vec<f64>,
}

#[derive(Debug, Serialize)]
pub struct SPK18 {
    data: Vec<f64>,
}

#[derive(Debug, Serialize)]
pub struct SPK19 {
    data: Vec<f64>,
}

#[derive(Debug, Serialize)]
pub struct SPK20 {
    data: Vec<f64>,
}

#[derive(Debug, Serialize)]
pub struct SPK21 {
    data: Vec<f64>,
}

#[derive(Debug, Serialize)]
pub struct SPKSegment {
    name: String,
    initial_epoch: f64,
    final_epoch: f64,
    target_code: i32,
    center_code: i32,
    frame_code: i32,
    spk_type: i32,
    data: Vec<f64>,
}
// impl to give SPKSegment from file and pointer to Summary rec

impl SPKSegment {
    fn reader(daf: &mut DAFFile, sumptr: u64) -> Result<DAFSegment> {
        let nameptr = sumptr + daf.namerec_offset;
        let data1 = daf.read_i32(sumptr + 32)? as u64;
        let data2 = daf.read_i32(sumptr + 36)? as u64;
        Ok(DAFSegment::SPK(SPKSegment {
            name: daf.read_string(nameptr, daf.nc)?,
            initial_epoch: daf.read_f64(sumptr)?,
            final_epoch: daf.read_f64(sumptr + 8)?,
            target_code: daf.read_i32(sumptr + 16)?,
            center_code: daf.read_i32(sumptr + 20)?,
            frame_code: daf.read_i32(sumptr + 24)?,
            spk_type: daf.read_i32(sumptr + 28)?,
            data: daf.read_f64vec(data1, data2)?,
        }))
    }
}

#[derive(Debug, Serialize)]
pub struct CKSegment {
    name: String,
    initial_sclk: f64,
    final_sclk: f64,
    instrument_code: i32,
    frame_code: i32,
    ck_type: i32,
    rates: bool,
    data: Vec<f64>,
}

impl CKSegment {
    fn reader(daf: &mut DAFFile, sumptr: u64) -> Result<DAFSegment> {
        let nameptr = sumptr + daf.namerec_offset;
        let data1 = daf.read_i32(sumptr + 32)? as u64;
        let data2 = daf.read_i32(sumptr + 36)? as u64;
        Ok(DAFSegment::CK(CKSegment {
            name: daf.read_string(nameptr, daf.nc)?,
            initial_sclk: daf.read_f64(sumptr)?,
            final_sclk: daf.read_f64(sumptr + 8)?,
            instrument_code: daf.read_i32(sumptr + 16)?,
            frame_code: daf.read_i32(sumptr + 20)?,
            ck_type: daf.read_i32(sumptr + 24)?,
            rates: (daf.read_i32(sumptr + 28)? == 1),
            data: daf.read_f64vec(data1, data2)?,
        }))
    }
}

#[derive(Debug, Serialize)]
pub struct BPCKSegment {
    name: String,
    initial_epoch: f64,
    final_epoch: f64,
    frame_id: i32,
    base_frame: i32,
    bpck_type: i32,
    data: Vec<f64>,
}
impl BPCKSegment {
    fn reader(daf: &mut DAFFile, sumptr: u64) -> Result<DAFSegment> {
        let nameptr = sumptr + daf.namerec_offset;
        let data1 = daf.read_i32(sumptr + 28)? as u64;
        let data2 = daf.read_i32(sumptr + 32)? as u64;
        Ok(DAFSegment::BPCK(BPCKSegment {
            name: daf.read_string(nameptr, daf.nc)?,
            initial_epoch: daf.read_f64(sumptr)?,
            final_epoch: daf.read_f64(sumptr + 8)?,
            frame_id: daf.read_i32(sumptr + 16)?,
            base_frame: daf.read_i32(sumptr + 20)?,
            bpck_type: daf.read_i32(sumptr + 24)?,
            data: daf.read_f64vec(data1, data2)?,
        }))
    }
}

#[derive(Debug, Serialize)]
pub enum DAFSegment {
    SPK(SPKSegment),
    CK(CKSegment),
    BPCK(BPCKSegment),
}

#[derive(Debug, Serialize)]
pub struct DAFHeader {
    name: String,
    comment: String,
    kind: String,
}

#[derive(Debug, Serialize)]
pub struct DAFData {
    header: DAFHeader,
    segments: Vec<DAFSegment>
}

impl DAFData {
    pub fn from_daffile(df: &mut DAFFile) -> Result<DAFData> {
        let header: DAFHeader = df.daf_header()?;
        let mut segments: Vec<DAFSegment> = Vec::new();

        for seg in df {
            match seg {
                Ok(s) => {segments.push(s);},
                Err(e) => {return Err(e);},
            }
        }

        Ok(DAFData {
            header,
            segments,
        })
    }
}
// TODO: add asserts to verify file data
