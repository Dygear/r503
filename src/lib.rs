#![no_std]
use core::mem::size_of;
use embedded_io_async::{Read, Write};
use heapless::Vec;

pub struct Driver<UART> {
    pub uart: UART,

    packet: Package,
}

// DOCUMENTATION: R503 https://github.com/adafruit/Adafruit-Fingerprint-Sensor-Library/files/10964885/R503.fingerprint.module.user.manual-V1.2.2.pdf
// DOUCMENTATION: R302 https://file.vishnumaiea.in/ds/module/biometric/R302-Fingerprint-Module-User-Manual.pdf

// # R503 Fingerprint Module User Manual
// # User Manual
// Hangzhou Grow Technology Co., Ltd.
// 2022.10 Ver: 1.2.2

#[allow(dead_code)]
const READY: u8 = 0x55;

// ## Preface & Declaration
// Thank you for you selection of R503 Fingerprint Identification Module of GROW.
// The Manual is targeted for hardware & software development engineer, covering module function, hardware and software interface etc. To ensure the developing process goes smoothly, it is highly recommended the Manual is read through carefully.
// Because of the products constantly upgraded and improved, module and the manual content may be changed without prior notice. If you want to get the latest information, please visit our company website (www.hzgrow.com).
// We have been trying our best to ensure you the correctness of the Manual. However, if you have any question or find error, feel free to contact us or the authorized agent. We would be very grateful.
// The Manual contains proprietary information of Hangzhou Grow Technology Co., Ltd., which shall not be used by or disclosed to third parties without the permission of GROW, nor for any reproduction and alteration of information without any associated warranties, conditions, limitations, or notices.
// No responsibility or liability is assumed by GROW for the application or use, nor for any infringements of patents or other intellectual property rights of third parties that may result from its use.

// # 1. Introduction
// Power DC: 3.3V
// Interface: UART(3.3V TTL logical level)
// Working current (Fingerprint acquisition): 20mA
// Matching Mode: 1:1 and 1:N
// Matching Time: 1:N<10ms/Fingerprint
// Standby current (finger detection): Typical touch standby voltage: 3.3V Average current: 2uA
/// Characteristic value size: 512 bytes
type CharacterData = [u8; 512];
// Baud rate: (9600*N)bps, N=1~6 (default N=6)
/// Template size: 1536 bytes
#[allow(dead_code)]
type TemplateData = [u8; 1536];
// Image acquiring time: <0.2s
// Image resolution: 508dpi
// Sensing Array: 192*192 pixel
type ImageData = [u8; 192 * 192];
// Detection Area Diameter: 15mm
// Storage capacity: 200
// Security level: 5 (1, 2, 3, 4, 5(highest)) FAR <0.001% FRR <1%
// Generate feature point time: < 500ms
// Starting time: =<50ms
// Working environment Temp: -20°C- +60°C
// Storage environment Temp: -40°C- +75°C
// RH: 10%-85%
// RH: <85%

// # Operation Principle
// Fingerprint processing includes two parts: fingerprint enrollment and fingerprint matching (the matching can be 1:1 or 1:N).
// When enrolling, user needs to enter the finger two times. The system will process the two time finger images, generate a template of the finger based on processing results and store the template.
// When matching, user enters the finger through optical sensor and system will generate a template of the finger and compare it with templates of the finger library.
// For 1:1 matching, system will compare the live finger with specific template designated in the Module; for 1:N matching, or searching, system will search the whole finger library for the matching finger.
// In both circumstances, system will return the matching result, success or failure.

// # 2. Hardware Interface
// ## Exterior Interface
// Connector: MX1.0--6P Thread: M25
// Product external diameter: 28mm
// Inner diameter: 23.5mm
// Height: 19mm

// # Serial Communication
// Connector: MX1.0--6P
// Pin      Color       Name            Description
// 1        Red         Power Supply    DC3.3V
// 2        Black       GND             Signal ground. Connected to power ground.
// 3        Yellow      TXD             Data output. TTL logical level.
// 4        Green/Brown RXD             Data input. TTL logical level.
// 5        Blue        WAKEUP          Finger Detection Signal.
// 6        White       3.3VT           Touch induction power supply, DC3—6V.
// Note: The line order has nothing to do with color.

// # Hardware connection
// The RX of the module is connected with the TX of the upper computer, and the TX of the module is connected with the RX of the upper computer.
// The IRQ signal can be connected with the middle fracture or IO port of the upper computer.
// To reduce the system standby power consumption, when the upper computer needs to use the fingerprint module, then power on the main power supply of the fingerprint module.
// At this time, the fingerprint module is powered on, and complete the corresponding instructions sent by the upper computer.
// When the upper computer does not need to use the fingerprint module, disconnect the fingerprint module from the main power supply.
// When the upper computer is in standby mode, in order to keep the finger touch detection, the touch power supply needs to be powered all the time.
// The working voltage of the touch power supply is 3V~5V, and the average current of the touch power supply is about 2uA.
// When there is no finger touch, the default touch sensing signal outputs high level; When a finger touches, the default touch sensing signal outputs low level.
// After detecting the touch sensing signal, the upper computer supplies power to the fingerprint module and the fingerprint module starts to work.
// The maximum response time of the touch function is about 120mS @vt =3.3V.
// When the module is not touched, the recalibration period is about 4.0sec; the touch signal output is CMOS output, and the output voltage is roughly the same as the input voltage.

// # Serial communication protocol
// The mode is semiduplex asychronism serial communication. And the default baud rate is 57600bps. User may set the baud rate in 9600~115200bps.
// Transferring frame format is 10 bit: the low-level starting bit, 8-bit data with the LSB first, and an ending bit. There is no check bit.

// # Power-on delay time
// At power on, it takes about 50ms for initialization.
// During this period, the Module can't accept commands for upper computer.
// After completing the initialization, the module will immediately send a byte (0x55) to the upper computer, indicating that the module can work normally and receive instructions from the upper computer.

// # Power Supply Requirements
// The power supply is DC +3.3V. The power input is allowed only after the R503 is properly connected.
// Electrical components of the R503 may be damaged if you insert or remove the cable (with the electric plug) when the cable is live.
// Ensure that the power supply is switched off when you insert or remove the cable.
// The R503 may not work properly due to poor power connections, short power off/on intervals, or excessive voltage drop pulses
// So pls (Yep, that's actually in the document) keep the power is stable.
// After the power is turned off, the power must be turned on at least two seconds later.

// # Ripple noise
// Since the power input of R503 is directly supplied to the image sensor and decoding chip.
// To ensure stable operation, pls (sic) use low ripple noise power input.
// It is recommended that the ripple noise not exceed 50mV (peak-to-peak).

// # 3. System Resources
// To address demands of different customer, Module system provides abundant resources at user's use.
// ## Notepad
// The system sets aside a 512-bytes memory (16 pages* 32 bytes) for user's notepad, where data requiring power-off protection can be stored. The host can access the page by instructions of PS_WriteNotepad and PS_Read Notepad.
// Note: when write on one page of the pad, the entire 32 bytes will be written in wholly covering the original contents.
// The user can run the module address or random number command to configure the unique matching between the module and the system.
// That is, the system identifies only the unique module; If a module of the same type is replaced, the system cannot access the system.
// ## Buffer
// The module RAM resources are as follows:
// An ImageBuffer: ImageBuffer
// 6 feature buffers: CharBuffer[1:6]
// All buffer contents are not saved without power.
// The user can read and write any buffer by instruction. CharBuffer can be used to store normal feature files or store template feature files.
// When uploading or downloading images through the UART port, only the high four bits of pixel bytes are used to speed up the transmission, that is, use gray level 16, two pixels are combined into one byte.
// (The high four bits are a pixel, the low four bits are a pixel in the next adjacent column of the same row, that is, two pixels are combined into one byte and transmitted).
// Since the image has 16 gray levels, when it is uploaded to PC for display (corresponding to BMP format), the gray level should be extended (256 gray levels, that is, 8-bit bitmap format).
// # Fingerprint Library
// System sets aside a certain space within Flash for fingerprint template storage, that's fingerprint library. The contents of the fingerprint database are protected by power-off, and the serial number of the fingerprint database starts from 0.
// Capacity of the library changes with the capacity of Flash, system will recognize the latter automatically. Fingerprint template's storage in Flash is in sequential order. Assume the fingerprint capacity N, then the serial number of template in library is 0, 1, 2, 3 ... N. User can only access library by template number.
// # System Configuration Parameters
// The system allows the user to individually modify a specified parameter value (by parameter serial number) by command; Refer to SetSysPara.
// After the upper computer sets the system parameter instructions, the system must be powered on again so that the module can work according to the new configuration.

#[repr(u8)]
#[derive(Clone, PartialEq, Eq)]
pub enum ParameterSetting {
    /// The Parameter controls the UART communication speed of the Module. Its value is an integer N, N= [1/2/4/6/12]. Corresponding baud rate is 9600*N bps.
    BaudRateControl = 4,
    /// The Parameter controls the matching threshold value of fingerprint searching and matching. Security level is divided into 5 grades, and corresponding value is 1, 2, 3, 4, 5. At level 1, FAR is the highest and FRR is the lowest; however at level 5, FAR is the lowest and FRR is the highest.
    SecurityLevel = 5,
    /// The parameter decides the max length of the transferring data package when communicating with upper computer. Its value is 0, 1, 2, 3, corresponding to 32 bytes, 64 bytes, 128 bytes, 256 bytes respectively.
    DataPackageLength = 6,
}

/// # Baud rate control (Parameter Number: 4)
/// The Parameter controls the UART communication speed of the Modul. Its value is an integer N, N= [1/2/4/6/12]. Cooresponding baud rate is 9600*N bps.
#[repr(u8)]
#[derive(Clone, Default, PartialEq, Eq)]
pub enum BaudRate {
    Rate9600 = 1,
    Rate19200 = 2,
    Rate38400 = 4,
    #[default]
    Rate57600 = 6,
    Rate115200 = 12,
}

/// # Security Level (Parameter Number: 5)
/// The Parameter controls the matching threshold value of fingerprint searching and matching. Security level is divided into 5 grades, and cooresponding value is 1, 2, 3, 4, 5. At level 1, FAR is the highest and FRR is the lowest; however at level 5, FAR is the lowest and FRR is the highest.
#[repr(u8)]
#[derive(Clone, PartialEq, Eq)]
pub enum SecurityLevel {
    Level1 = 1,
    Level2 = 2,
    Level3 = 3,
    Level4 = 4,
    Level5 = 5,
}

/// # Data package length (Parameter Number: 6)
/// The parameter decides the max length of the transferring data package when communicating with upper computer. Its value is 0, 1, 2, 3, corresponding to 32 bytes, 64 bytes, 128 bytes, 256 bytes respectively.
#[repr(u8)]
#[derive(Clone, Default, PartialEq, Eq)]
pub enum PacketLength {
    Bytes32 = 0,
    Bytes64 = 1,
    Bytes128 = 2,
    #[default]
    Bytes256 = 3,
}

// # System status register
// System status register indicates the current operation status of the Module. Its length is 1 word, and can be read via instruction ReadSysPara. Definition of the register is as follows:
// Bit Number   Description Notes
// 15  - 4      Reserved    Reserved
// 3            ImgBufStat  1 = Image buffer contains valid image.
// 2            PWD         1 = Verified device's handshaking password.
// 1            Pass        1 = Find the matching finger; 0 = wrong finger;
// 0            Busy        1 = Sstem is executing commands; 0 = system is free;

#[repr(u16)]
#[derive(Clone, Default, PartialEq, Eq)]
pub enum SystemRegister {
    Busy = 0b0000_0000_0000_0001,       // 0x01 (1)
    Pass = 0b0000_0000_0000_0010,       // 0x02 (2)
    Pwd = 0b0000_0000_0000_0100,        // 0x03 (3)
    ImgBufStat = 0b0000_0000_0000_1000, // 0x04 (4)
    #[default]
    Reserved = 0b1111_1111_1111_0000,
}

impl SystemRegister {
    pub fn from(value: u16) -> Self {
        match value {
            0b0000_0000_0000_0001 => Self::Busy,
            0b0000_0000_0000_0010 => Self::Pass,
            0b0000_0000_0000_0100 => Self::Pwd,
            0b0000_0000_0000_1000 => Self::ImgBufStat,
            _ => Self::Reserved,
        }
    }
}

/// ## Module password
/// The default password of the module is `0x00000000`. If the default password is modified, after the module is powered on,the first instruction of the upper computer to communicate with the module must be verify password. Only after the password verification is passed, the module will enter the normal working state and receive other instructions.
/// The new modified password is stored in Flash and remains at power off.(the modified password cannot be obtained through the communication instruction. If forgotten by mistake, the module cannot communicate, please use with caution)
/// Refer to instruction SetPwd and VfyPwd.
pub type Password = u32;
const PASSWORD: Password = 0x00000000;
// ## Module address
// Each module has an identifying address. When communicating with upper computer, each instruction/data is transferred in data package form, which contains the address item. Module system only responds to data package whose address item value is the same with its identifying address.
// The address length is 4 bytes, and its default factory value is `0xFFFFFFFF`. User may modify the address via instruction SetAddr. The new modified address remains at power off.
// ## Random number generator
// Module integrates a hardware 32-bit random number generator (RNG) (without seed). Via instruction GetRandomCode, system will generate a random number and upload it.
// # Features and templates
// The chip has one image buffer and six feature file buffers,all buffer contents are not saved after power failure.
// A template can be composed of 2-6 feature files. The more feature files in the synthesis template, the better the quality of the fingerprint template.
// It is recommended to take at least four templates to synthesize features.

// # 4 Communication Protocol
// The protocol defines the data exchanging format when R503 series communicates with upper computer.
// The protocol and instruction sets apples for both UART communication mode.
// Baud rate 57600, data bit 8, stop bit 1, parity bit none.
// ## 4.1 Data package format
// When communicating, the transferring and receiving of command/data/result are all wrapped in data package format. For multi-bytes, the high byte precedes the low byte (for example, a 2 bytes 00 06 indicates 0006, not 0600).
// ### Data package format
//      Header
//      Adder
//      Package identifier
//      Package length
//      Package content (instuction/data/Parameter)
//      Checksum
// ### Definition of Data package
/// Name: Header
/// Symbol: Start
/// Length: 2 Bytes
/// Description: Fixed value of 0xEF01; High byte transferred first.
pub type Header = u16;
pub const HEADER: Header = 0xEF01;

/// Name: Adder
/// Symbol: ADDER
/// Length: 4 bytes
/// Description: Default value is 0xFFFFFFFF, which can be modified by command. High byte transferred first and at wrong adder value, module will reject to transfer.
pub type Address = u32;
pub const ADDRESS: Address = 0xFFFFFFFF;

// Name: Package identifier
// Symbol: PID
// Length: 1 Byte
// Description:
//      0x01: Command packet;
//      0x02: Data packet; Data packet shall not appear alone in executing process, must follow command packet or acknowledge packet.
//      0x07: Acknowledge packet;
//      0x08: End of Data packet.
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Identifier {
    Command = 0x01,
    Data = 0x02,
    Acknowledge = 0x07,
    End = 0x08,
}
impl From<Identifier> for u8 {
    fn from(item: Identifier) -> Self {
        item as u8
    }
}

/// Name: Package length
/// Symbol: LENGTH
/// Length: 2 Bytes
/// Description: Refers to the length of package content (command packets and data packets) plus the length of Checksum (2 bytes). Unit is byte. Max length is 256 bytes. And high byte is transferred first.
pub type Length = u16;

// Name: Package contents
// Symbol: DATA
// Length: -
// Description: It can be commands, data, command's parameters, acknowledge result, etc. (fingerprint character value, template are all deemed as data);
pub struct Data {
    /// The first and maybe only thing is the instruction
    pub instruction: Instruction,
    /// The payload of the document.
    pub payload: Payload,
}

impl Default for Data {
    fn default() -> Self {
        Self {
            instruction: Instruction::SoftRst,
            payload: Payload::None,
        }
    }
}

impl Data {
    pub fn len(&self) -> usize {
        let mut len = size_of::<Instruction>();
        len += self.payload.len();
        len
    }

    pub fn is_empty(&self) -> bool {
        false
    }
}

#[allow(clippy::large_enum_variant)]
pub enum Payload {
    None,
    VfyPwd(Password),
    SetPwd(Password),
    SetAdder(Address),
    SetSysPara(ParameterSetting, u8),
    ReadSysPara,
    TempleteNum,
    ReadIndexTable(IndexPage),
    GenImg,
    Img2Tz(BufferID),
    UpImage,
    DownImage,
    GenChar(BufferID),
    RegModel,
    UpChar(BufferID),
    DownChar(BufferID),
    Store(BufferID, IndexPage),
    LoadChar(BufferID, IndexPage),
    DeleteChar(BufferID, u8),
    Empty,
    Match,
    Search(BufferID, u16, u16),
    GetImageEx,
    Cancel,
    HandShake,
    CheckSensor,
    GetAlgVer,
    GetFwVer,
    ReadProdInfo,
    SoftRst,
    AuraLedConfig(LightPattern, Speed, Color, Times),
    AutoEnroll(IndexPage, bool, bool, bool, bool),
    AutoIdentify(SafeGrade, u8, u8, u8, bool),
    GetRandomCode,
    ReadInfPage,
    WriteNotepad(NotePageNum, Page),
    ReadNotepad(NotePageNum),
}

impl Payload {
    pub fn len(&self) -> usize {
        match self {
            Self::None => size_of::<()>(),
            Self::VfyPwd(_) => size_of::<Password>(),
            Self::SetPwd(_) => size_of::<Password>(),
            Self::SetAdder(_) => size_of::<Address>(),
            Self::SetSysPara(_, _) => size_of::<(ParameterSetting, u8)>(),
            Self::ReadSysPara => size_of::<()>(),
            Self::TempleteNum => size_of::<()>(),
            Self::ReadIndexTable(_) => size_of::<IndexPage>(),
            Self::GenImg => size_of::<()>(),
            Self::Img2Tz(_) => size_of::<BufferID>(),
            Self::UpImage => size_of::<()>(),
            Self::DownImage => size_of::<()>(),
            Self::GenChar(_) => size_of::<BufferID>(),
            Self::RegModel => size_of::<()>(),
            Self::UpChar(_) => size_of::<BufferID>(),
            Self::DownChar(_) => size_of::<BufferID>(),
            Self::Store(_, _) => size_of::<(BufferID, IndexPage)>(),
            Self::LoadChar(_, _) => size_of::<(BufferID, IndexPage)>(),
            Self::DeleteChar(_, _) => size_of::<(BufferID, u8)>(),
            Self::Empty => size_of::<()>(),
            Self::Match => size_of::<()>(),
            Self::Search(_, _, _) => size_of::<(BufferID, u16, u16)>(),
            Self::GetImageEx => size_of::<()>(),
            Self::Cancel => size_of::<()>(),
            Self::HandShake => size_of::<()>(),
            Self::CheckSensor => size_of::<()>(),
            Self::GetAlgVer => size_of::<()>(),
            Self::GetFwVer => size_of::<()>(),
            Self::ReadProdInfo => size_of::<()>(),
            Self::SoftRst => size_of::<()>(),
            Self::AuraLedConfig(_, _, _, _) => size_of::<(LightPattern, Speed, Color, Times)>(),
            Self::AutoEnroll(_, _, _, _, _) => size_of::<(IndexPage, bool, bool, bool, bool)>(),
            Self::AutoIdentify(_, _, _, _, _) => size_of::<(SafeGrade, u8, u8, u8, bool)>(),
            Self::GetRandomCode => size_of::<()>(),
            Self::ReadInfPage => size_of::<()>(),
            Self::WriteNotepad(_, _) => size_of::<(NotePageNum, Page)>(),
            Self::ReadNotepad(_) => size_of::<NotePageNum>(),
        }
    }

    pub fn is_empty(&self) -> bool {
        matches!(
            self,
            Self::None
                | Self::ReadSysPara
                | Self::TempleteNum
                | Self::GenImg
                | Self::UpImage
                | Self::DownImage
                | Self::RegModel
                | Self::Empty
                | Self::Match
                | Self::GetImageEx
                | Self::Cancel
                | Self::HandShake
                | Self::CheckSensor
                | Self::GetAlgVer
                | Self::GetFwVer
                | Self::ReadProdInfo
                | Self::SoftRst
                | Self::GetRandomCode
                | Self::ReadInfPage
        )
    }
}

// Name: Checksum
// Symbol: SUM
// Length: 2 Bytes
// Description: The arithmetic sum of package identifier, package length and all package contens. Overflowing bits are omitted. high byte is transferred first.
pub type Sum = u16;

pub struct Package {
    /// See Name: Header
    pub header: Header,
    /// See Name: Adder
    pub address: Address,
    /// See Name: Package identifier
    pub identifier: Identifier,
    /// See Name: Package length
    pub length: Length,
    /// See Name: Package contents
    pub contents: Data,
    /// See Name: Checksum
    pub checksum: Sum,
}

impl Package {
    /// Header
    pub fn get_header(&self) -> Header {
        self.header
    }

    /// Get Address
    pub fn get_address(&self) -> Address {
        self.address
    }
    pub fn set_address(&mut self, address: Address) -> &Self {
        self.address = address;
        self
    }

    /// Get Identifier
    pub fn get_identifier(&self) -> Identifier {
        self.identifier
    }
    pub fn set_identifier(&mut self, identifier: Identifier) -> &Self {
        self.identifier = identifier;
        self
    }

    /// Gets the length of the package in bytes.
    pub fn length(&mut self) -> Length {
        let mut length: Length = 0;
        length += size_of::<Header>() as u16;
        length += size_of::<Address>() as u16;
        length += size_of::<Identifier>() as u16;
        length += size_of::<Length>() as u16;
        // TODO: We actually want to call self.contents.len() to get the length that way.
        // Right now, this is going to give the max size of a variant of Data.
        length += self.contents.len() as u16;
        length += size_of::<Sum>() as u16;

        // Yep. It's a u16, but has to be less than or equal to 256 bytes.
        assert!(length <= 256);

        self.length = length;
        length
    }

    /// Get Contents
    pub fn get_contents(&self) -> &Data {
        &self.contents
    }
    pub fn set_contents(&mut self, contents: Data) -> &Self {
        self.contents = contents;
        self
    }

    /// Calculates the checksum of all of the bytes in the Package.
    /// Does so by looking at each byte and adding it's value to our Sum type.
    pub fn checksum(&mut self) -> Sum {
        let mut checksum: Sum = 0;

        // Identifier
        checksum += self.identifier as u16;
        // Length
        checksum += get_u16_as_u16_parts(self.length)[0];
        checksum += get_u16_as_u16_parts(self.length)[1];
        // TODO: Contents

        self.checksum = checksum;
        checksum
    }
    pub fn get_checksum(&self) -> Sum {
        self.checksum
    }

    /// Building function to quickly get the bytes setup correct.
    pub fn build(identifier: Identifier, instruction: Instruction, payload: Payload) -> Self {
        let mut package = Self {
            identifier,
            contents: Data {
                instruction,
                payload,
            },
            ..Default::default()
        };
        package.length();
        package.checksum();

        package
    }

    /// Get the bytes of the struct cleanly.
    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                self as *const Self as *const u8,
                core::mem::size_of::<Package>(),
            )
        }
    }
}

impl Default for Package {
    fn default() -> Self {
        Self {
            header: HEADER,
            address: ADDRESS,
            identifier: Identifier::Acknowledge,
            length: 12,
            contents: Data::default(),
            checksum: Sum::default(),
        }
    }
}

type NotePageNum = u8;

// # Check and acknowledgement of data package
// Note: Commands shall only be sent from upper computer to the Module, and the Module acknowledges the commands.
// Upon receipt of commands, Module will report the commands execution status and results to upper computer through acknowledge packet. Acknowledge packet has parameters and may also have following data packet. Upper computer can't ascertain Module's package receiving status or command execution results unless through acknowledge packet sent from Module. Acknowledge packet includes 1 byte confirmation code and maybe also the returned parameter.
// Confirmation code's definition is:

#[repr(u8)]
#[derive(Debug, Default, PartialEq)]
pub enum ConfirmationCode {
    /// 0 = Commad Execution Complete;
    Success = 0x00,
    /// 1 = Error When Receiving Data Package;
    ErrorReceivingPacket = 0x01,
    /// 2 = No Finger on the Sensor;
    NoFingerOnSensor = 0x02,
    /// 3 = Fail to Enroll the Finger;
    FailToEnrollFinger = 0x03,
    /// 6 = Fail to Generate Character File Due to the Over-disorderly Fingerprint Image;
    FailToGenerateCharacterOverDisorderlyFingerprintImage = 0x06,
    /// 7 = Fail to Generate Character File due to Lackness of Character Point or Over-smallness of Fingerprint Image;
    FailToGenerateCharacterLacknessOfCharacterPointOrOverSmallness = 0x07,
    /// 8 = Finger Doesn't Match;
    FailFingerDoesntMatch = 0x08,
    /// 9 = Fail to Find the Matching Finger;
    FailToFindMatchingFinger = 0x09,
    /// 10 = Fail to Combine the Character Files;
    FailToCombineCharacterFiles = 0x0A,
    /// 11 = Addressing PageID is Beyond the Finger Library;
    AddressingPageIDIsBeyoundTheFingerLibary = 0x0B,
    /// 12 = Error When Reading Template from Library or the Template is Invalid;
    ErrorWhenReadingTemplateFromLibararORTemplateIsInvalid = 0x0C,
    /// 13 = Error When Uploading Template;
    ErrorWhenUploadingTemplate = 0x0D,
    /// 14 = Module can't receive the following data packages;
    ModuleCantReceivingTheFollowingDataPackages = 0x0E,
    /// 15 = Error when uploading image;
    ErrorWhenUploadingImage = 0x0F,
    /// 16 = Fail to delete the template;
    FailToDeleteTheTemplate = 0x10,
    /// 17 = Fail to clear finger library;
    FailToClearFingerLibary = 0x11,
    /// 19 = Wrong password;
    WrongPassword = 0x13,
    /// 21 = Fail to generate the image for the lackness of valid primary image;
    FailToGenerateImageLacknessOfValidPrimaryImage = 0x15,
    /// 24 = Error when writing flash;
    ErrorWhenWritingFlash = 0x18,
    /// 25 = No definition error;
    NoDefinitionError = 0x19,
    /// 26 = Invalid register number;
    InvalidRegisterNumber = 0x1A,
    /// 27 = Incorrect configuration of register;
    IncorrectConfigurationOfRegister = 0x1B,
    /// 28 = Wrong notepad page number;
    WrongNotepadPageNumber = 0x1C,
    /// 29 = Fail to operate the communication port;
    FailToOperateTheCommunicationPort = 0x1D,
    /// 31 = The fingerprint libary is full;
    FingerPrintLibaryFull = 0x1F,
    /// 32 = The address code is incorrect;
    AddressIncorrect = 0x20,
    /// 33 = The password must be verified;
    MustVerifyPassword = 0x21,
    /// 34 = The fingerprint template is empty;
    FingerTemplateEmpty = 0x22,
    /// 36 = The fingerprint library is empty;
    FingerLibaryEmpty = 0x24,
    /// 38 = Timeout;
    Timeout = 0x26,
    /// 39 = The fingerprints already exist;
    FingerAlreadyExists = 0x27,
    /// 41 Sensor hardware error;
    SensorHardwareError = 0x29,
    /// 252 = Unsupported command;
    UnsupportedCommand = 0xFC,
    /// 253 = Hardware Error;
    HardwareError = 0xFD,
    /// 254 = Command execution failure;
    CommandExecutionFailure = 0xFE,
    /// 255 Others: System Reserved; (And Default for this Rust Lib);
    #[default]
    SystemReserved = 0xFF,
}

impl ConfirmationCode {
    pub fn from(value: u8) -> Self {
        match value {
            0x00 /*   0 */ => Self::Success,
            0x01 /*   1 */ => Self::ErrorReceivingPacket,
            0x02 /*   2 */ => Self::NoFingerOnSensor,
            0x03 /*   3 */ => Self::FailToEnrollFinger,
            0x06 /*   6 */ => Self::FailToGenerateCharacterOverDisorderlyFingerprintImage,
            0x07 /*   7 */ => Self::FailToGenerateCharacterLacknessOfCharacterPointOrOverSmallness,
            0x08 /*   8 */ => Self::FailFingerDoesntMatch,
            0x09 /*   9 */ => Self::FailToFindMatchingFinger,
            0x0A /*  10 */ => Self::FailToCombineCharacterFiles,
            0x0B /*  11 */ => Self::AddressingPageIDIsBeyoundTheFingerLibary,
            0x0C /*  12 */ => Self::ErrorWhenReadingTemplateFromLibararORTemplateIsInvalid,
            0x0D /*  13 */ => Self::ErrorWhenUploadingTemplate,
            0x0E /*  14 */ => Self::ModuleCantReceivingTheFollowingDataPackages,
            0x0F /*  15 */ => Self::ErrorWhenUploadingImage,
            0x10 /*  16 */ => Self::FailToDeleteTheTemplate,
            0x11 /*  17 */ => Self::FailToClearFingerLibary,
            0x13 /*  19 */ => Self::WrongPassword,
            0x15 /*  21 */ => Self::FailToGenerateImageLacknessOfValidPrimaryImage,
            0x18 /*  24 */ => Self::ErrorWhenWritingFlash,
            0x19 /*  25 */ => Self::NoDefinitionError,
            0x1A /*  26 */ => Self::InvalidRegisterNumber,
            0x1B /*  27 */ => Self::IncorrectConfigurationOfRegister,
            0x1C /*  28 */ => Self::WrongNotepadPageNumber,
            0x1D /*  29 */ => Self::FailToOperateTheCommunicationPort,
            0x1F /*  31 */ => Self::FingerPrintLibaryFull,
            0x20 /*  32 */ => Self::AddressIncorrect,
            0x21 /*  33 */ => Self::MustVerifyPassword,
            0x22 /*  34 */ => Self::FingerTemplateEmpty,
            0x24 /*  36 */ => Self::FingerLibaryEmpty,
            0x26 /*  38 */ => Self::Timeout,
            0x27 /*  39 */ => Self::FingerAlreadyExists,
            0x29 /*  41 */ => Self::SensorHardwareError,
            0xFC /* 252 */ => Self::UnsupportedCommand,
            0xFD /* 253 */ => Self::HardwareError,
            0xFE /* 254 */ => Self::CommandExecutionFailure,
            _    /* 255 */ => Self::SystemReserved,
        }
    }
}

// # 5. Module Instruction System
// R30X series provide 23 instructions. R50X series provide 33 instructions. Through combination of different instructions, application program may realize muti finger authentication functions. All commands/data are transferred in package format. Refer to 5.1 for the detailed information of package.

impl<UART> Driver<UART>
where
    UART: Read + Write,
{
    /// You must provide a `uart` that can read and write.
    /// You can provide an address, or it will use the default address.
    pub fn new(&mut self, uart: UART, address: Option<Address>) -> &mut Self {
        self.uart = uart;
        self.packet.address = match address {
            Some(address) => address,
            None => ADDRESS,
        };
        self
    }

    // # System-related instructions

    /// Verify passwoard - VfyPwd
    /// Description: Verify Module's handshaking password.
    /// Input Parameter: Password (4 bytes)
    /// Return Parameter: Confirmation code (1 byte)
    ///     0x00: Correct password;
    ///     0x01: Error when receiving package;
    ///     0x13: Wrong password;
    /// Instruction code: 0x13
    pub fn vfy_pwd(&mut self, password: Option<Password>) -> Package {
        let package = Package::build(
            Identifier::Command,
            Instruction::VfyPwd,
            Payload::VfyPwd(match password {
                Some(password) => password,
                None => PASSWORD,
            }),
        );

        package
    }

    /// Set password - SetPwd
    /// Description: Set Module's handshaking password.
    /// Input Parameter: Password (4 bytes)
    /// Return Parameter: Confirmation code (1 byte)
    ///     0x00: Password setting complete;
    ///     0x01: Error when receiving package;
    ///     0x18: Error when writing FLASH;
    ///     0x21: Have to verify password;
    /// Instruction code: 0x12
    pub fn set_pwd(&mut self, password: Option<Password>) -> Package {
        let package = Package::build(
            Identifier::Command,
            Instruction::SetPwd,
            Payload::SetPwd(match password {
                Some(password) => password,
                None => PASSWORD,
            }),
        );

        package
    }

    /// Set Module address - SetAdder
    /// Description: Set Module address.
    /// Input Parameter: Addr
    /// Return Parameter: Confirmation code (1 byte)
    ///     0x00: Address setting complete;
    ///     0x01: Error when receiving package;
    ///     0x18: Error when writing FLASH;
    /// Instruction code: 0x15
    pub fn set_adder(&mut self, address: Address) -> Package {
        let package = Package::build(
            Identifier::Command,
            Instruction::SetAdder,
            Payload::SetAdder(address),
        );

        package
    }

    /// Set module system's basic parameter - SetSysPara
    /// Description: Operation parameter settings.
    /// Input Parameter: Parameter number + Contents;
    /// Return Parameter: Confirmation code (1 byte)
    ///     0x00: Parameter setting complete;
    ///     0x01: Error when receiving package;
    ///     0x18: Error when writing FLASH;
    ///     0x1A: Wrong register number;
    /// Instruction code: 0x0E
    pub fn set_sys_para(&mut self, parameter: ParameterSetting, content: u8) -> Package {
        let package = Package::build(
            Identifier::Command,
            Instruction::SetSysPara,
            Payload::SetSysPara(parameter, content),
        );

        package
    }

    /// Read system Parameter - ReadSysPara
    /// Description: Read Module's status register and system basic configuration parameters;
    /// Input Parameter: none
    /// Return Parameter: Confirmation code (1 byte) + basic parameter (16 bytes)
    ///     0x00: Read complete;
    ///     0x01: Error when receiving package;
    ///     0x18: Error when writing FLASH;
    /// Instuction code: 0x0F
    pub fn read_sys_para(&mut self) -> Package {
        let package = Package::build(
            Identifier::Command,
            Instruction::ReadSysPara,
            Payload::ReadSysPara,
        );

        package
    }

    /// Read valid template number - TempleteNum
    /// Description: read the current valid template number of the Module
    /// Input Parameter: none
    /// Return Parameter: Confirmation code (1 byte)，template number:N
    ///     0x00: Read success;
    ///     0x01: Error when receiving package;
    /// Instuction code: 0x1D
    pub fn templete_num(&mut self) -> Package {
        let package = Package::build(
            Identifier::Command,
            Instruction::TempleteNum,
            Payload::TempleteNum,
        );

        package
    }

    /// Read fingerprint template index table - ReadIndexTable(0x1F)
    /// Description: Read the fingerprint template index table of the module, read the index table of the fingerprint template up to 256 at a time (32 bytes)
    /// Input Parameter: Index page
    /// Return Parameter: Confirmation code+Fingerprint template index table
    ///     0x00: Read complete;
    ///     0x01: Error when receiving package;
    /// Instuction code: 0x1F
    pub fn read_index_table(&mut self, index_page: IndexPage) -> Package {
        let package = Package::build(
            Identifier::Command,
            Instruction::ReadIndexTable,
            Payload::ReadIndexTable(index_page),
        );

        package
    }

    // # Fingerprint-processing instructions

    /// To collect finger image - GenImg
    /// Description: detecting finger and store the detected finger image in ImageBuffer while returning successfully confirmation code; If there is no finger, returned confirmation code would be "can't detect finger".
    /// Note: The difference between GetImageEx and GetImage instruction:
    ///     GetImage: When the image quality is poor, return confirmation code 0x00 (the image is successfully captured).
    ///     GetImageEx: When image quality is poor, return confirmation code 0x07 (image quality is too poor).
    /// Input Parameter: none
    /// Return Parameter: Confirmation code (1 byte)
    ///     0x00: Finger collection successs;
    ///     0x01: Error when receiving package;
    ///     0x02: Can't detect finger;
    ///     0x03: Fail to collect finger;
    /// Instuction code: 0x01
    pub fn gen_img(&mut self) -> Package {
        let package = Package::build(Identifier::Command, Instruction::GenImg, Payload::GenImg);

        package
    }

    /// I don't know if this function still exists.
    /// To generate character file from image - Img2Tz
    /// Description: to generate character file from the original finger image in ImageBuffer and store the file in CharBuffer1 or CharBuffer2.
    /// Input Parameter: BufferID (character file buffer number)
    /// Return Parameter: Confirmation code (1 byte)
    ///     0x00: Generate character file complete;
    ///     0x01: Error when receiving package;
    ///     0x06: Fail to generate character file due to the over-disorderly fingerprint image;
    ///     0x07: Fail to generate character file due to lackness of character point or over-smallness of fingerprint image;
    ///     0x15: Fail to generate the image for the lackness of valid primary image;
    /// Instuction code: 0x02
    pub fn img2_tz(&mut self, buffer_id: BufferID) -> Package {
        let package = Package::build(
            Identifier::Command,
            Instruction::GenChar,
            Payload::GenChar(buffer_id),
        );

        package
    }

    /// Upload image - UpImage
    /// Description: to upload the image in Img_Buffer to upper computer.
    /// Input Parameter: none
    /// Return Parameter: Confirmation code (1 byte)
    ///     0x00: Ready to transfer the following data packet;
    ///     0x01: Error when receiving package;
    ///     0x0F: Fail to transfer the following data packet;
    /// Instuction code: 0x0A
    /// Note: The upper computer sends the command packet, the module sends the acknowledge packet first, and then sends several data packet.
    /// Note: Packet Bytes N is determined by Packet Length. The value is 128 Bytes before delivery.
    pub fn up_image(&mut self) -> Package {
        let package = Package::build(Identifier::Command, Instruction::UpImage, Payload::UpImage);

        package
    }

    /// Download the image - DownImage
    /// Description: to download image from upper computer to Img_Buffer.
    /// Input Parameter: none
    /// Return Parameter: Confirmation code (1 byte)
    ///     0x00: Ready to transfer the following data packet;
    ///     0x01: Error when receiving package;
    ///     0x0E: Fail to transfer the following data packet;
    /// Instuction code: 0x0B
    /// Note: The upper computer sends the command packet, the module sends the acknowledge packet first, and then sends several data packet.
    /// Note: Packet Bytes N is determined by Packet Length. The value is 128 Bytes before delivery.
    pub fn down_image(&mut self, _image: ImageData) -> Package {
        let package = Package::build(
            Identifier::Command,
            Instruction::DownImage,
            Payload::DownImage,
        );

        package
    }

    /// To generate character file from image - GenChar
    /// Description: to generate character file from the original finger image in ImageBuffer
    /// Input Parameter: BufferID (character file buffer number)
    /// Return Parameter: Confirmation code (1 byte)
    ///     0x00: Generate character file complete;
    ///     0x01: Error when receiving package;
    ///     0x06: Fail to generate character file due to the over-disorderly fingerprint image;
    ///     0x07: Fail to generate character file due to lackness of character point or over-smallness of fingerprint image;
    ///     0x15: Fail to generate the image for the lackness of valid primary image;
    /// Instruction code: 0x02
    pub fn gen_char(&mut self, buffer_id: BufferID) -> Package {
        let package = Package::build(
            Identifier::Command,
            Instruction::GenChar,
            Payload::GenChar(buffer_id),
        );

        package
    }

    /// To generate template - RegModel
    /// Description: To combine information of character files from CharBuffer1 and CharBuffer2 and generate a template which is stroed back in both CharBuffer1 and CharBuffer2.
    /// Input Parameter: none
    /// Return Parameter:Confirmation code (1 byte)
    ///     0x00: Operation success;
    ///     0x01: Error when receiving package;
    ///     0x0A: Fail to combine the character files. That's, the character files don't belong to one finger.
    /// Instuction code: 0x05
    pub fn reg_model(&mut self) -> Package {
        let package = Package::build(
            Identifier::Command,
            Instruction::RegModel,
            Payload::RegModel,
        );

        package
    }

    /// To upload character or template - UpChar
    /// Description: Upload the data in the template buffer ModelBuffer to the upper computer.
    /// Input Parameter: BufferID (Buffer number)
    /// Return Parameter: Confirmation code (1 byte)
    ///     0x00: Ready to transfer the following data packet;
    ///     0x01: Error when receiving package;
    ///     0x0D: Eerror when uploading template;
    ///     0x0F: Can not receive the following data packet;
    /// Instuction code: 0x08
    /// Note: This command don't need to use the CharBufferID, so the CharBufferID can be any value between 1 and 6.
    /// Note: The upper computer sends the command packet, the module sends the acknowledge packet first, and then sends several data packet.
    /// Note: Packet Bytes N is determined by Packet Length. The value is 128 Bytes before delivery.
    /// Note: The instruction doesn't affect buffer contents.
    pub fn up_char(&mut self, buffer_id: BufferID) -> Package {
        let package = Package::build(
            Identifier::Command,
            Instruction::UpChar,
            Payload::UpChar(buffer_id),
        );

        package
    }

    /// Download template - DownChar
    /// Description: upper computer download template to module buffer
    /// Input Parameter: CharBufferID (Buffer number)
    /// Return Parameter: Confirmation code (1 byte)
    ///     0x00: Ready to transfer the following data packet;
    ///     0x01: Error when receiving package;
    ///     0x0E: Can not receive the following data packet
    /// Instuction code: 0x09
    /// Note: The upper computer sends the command packet, the module sends the acknowledge packet first, and then sends several data packet.
    /// Note: Packet Bytes N is determined by Packet Length. The value is 128 Bytes before delivery.
    /// Note: The instruction doesn't affect buffer contents.
    pub fn down_char(&mut self, buffer_id: BufferID, template: CharacterData) -> Package {
        let package = Package::build(
            Identifier::Command,
            Instruction::DownChar,
            Payload::DownChar(buffer_id),
        );

        // TODO: This one is going to be a doozy. Going to have to send muliple packets after the first one.
        let _silence = template;

        package
    }

    /// To store template - Store
    /// Description: to store the template of specified buffer (Buffer1/Buffer2) at the designated location of Flash library.
    /// Input Parameter: CharBufferID, ModelID
    /// Return Parameter: Confirmation code (1 byte)
    ///     0x00: Storage success;
    ///     0x01: Error when receiving package;
    ///     0x0B: Addressing ModelID is beyond the finger library;
    ///     0x18: Error when writing Flash.
    /// Instuction code: 0x06
    /// Note: CharBufferID is filled with 0x01
    pub fn store(&mut self, buffer_id: BufferID, model_id: IndexPage) -> Package {
        // TODO: This one is a little funky. _model_id param expects a [u8; 2].
        // The first byte being the page number, (0-3)
        // The second byte being the index in that page. (0-255)
        let package = Package::build(
            Identifier::Command,
            Instruction::Store,
            Payload::Store(buffer_id, model_id),
        );

        package
    }

    /// To read template from Flash library - LoadChar
    /// Description: to load template at the specified location (PageID) of Flash library to template buffer CharBuffer1/CharBuffer2
    /// Input Parameter: CharBufferID, ModelID
    /// Return Parameter: Confirmation code (1 byte)
    ///     0x00: Load success;
    ///     0x01: Error when receiving package;
    ///     0x0C: Error when reading template from library or the readout template is invalid;
    ///     0x0B: Addressing ModelID is beyond the finger library;
    /// Instuction code: 07H
    /// Note: CharBufferID is filled with 0x01
    pub fn load_char(&mut self, buffer_id: BufferID, model_id: IndexPage) -> Package {
        // TODO: This one is a little funky. _model_id param expects a [u8; 2].
        // The first byte being the page number, (0-3)
        // The second byte being the index in that page. (0-255)
        let package = Package::build(
            Identifier::Command,
            Instruction::LoadChar,
            Payload::LoadChar(buffer_id, model_id),
        );

        package
    }

    /// To delete template - DeleteChar
    /// Description: to delete a segment (N) of templates of Flash library started from the specified location (or PageID);
    /// Input Parameter: StartID + Num
    /// Return Parameter: Confirmation code (1 byte)
    ///     0x00: Delete success;
    ///     0x01: Error when receiving package;
    ///     0x10: Failed to delete templates;
    ///     0x18: Error when write FLASH;
    /// Instuction code: 0x0C
    pub fn delete_char(&mut self, start_id: BufferID, num: u8) -> Package {
        let package = Package::build(
            Identifier::Command,
            Instruction::DeleteChar,
            Payload::DeleteChar(start_id, num),
        );

        package
    }

    /// To empty finger library - Empty
    /// Description: to delete all the templates in the Flash library
    /// Input Parameter: none
    /// Return Parameter: Confirmation code (1 byte)
    ///     0x00: Empty success;
    ///     0x01: Error when receiving package;
    ///     0x11: Fail to clear finger library;
    ///     0x18: Error when write FLASH;
    /// Instuction code: 0x0D
    pub fn empty(&mut self) -> Package {
        let package = Package::build(Identifier::Command, Instruction::Empty, Payload::Empty);

        package
    }

    /// To carry out precise matching of two finger templates - Match
    /// Description: Compare the recently extracted character with the templates in the ModelBuffer, providing matching results.
    /// Input Parameter: none
    /// Return Parameter: Confirmation code (1 byte)，matching score.
    ///     0x00: templates of the two buffers are matching!
    ///     0x01: error when receiving package;
    ///     0x08: templates of the two buffers aren't matching;
    /// Instuction code: 0x03
    /// Note: The instruction doesn't affect the contents of the buffers.
    pub fn r#match(&mut self) -> Package {
        let package = Package::build(Identifier::Command, Instruction::Match, Payload::Match);

        package
    }

    /// To search finger library - Search
    /// Description: to search the whole finger library for the template that matches the one in CharBuffer1 or CharBuffer2. When found, PageID will be returned.
    /// Input Parameter: CharBufferID + StartID + Num
    /// Return Parameter: Confirmation code + ModelID (template number) + MatchScore
    ///     0x00: found the matching finer;
    ///     0x01: error when receiving package;
    ///     0x09: No matching in the library (both the PageID and matching score are 0);
    /// Instuction code: 0x04
    /// Note: The instruction doesn't affect the contents of the buffers.
    pub fn search(
        &mut self,
        buffer_id: BufferID,
        start_page: u16,
        num: u16,
    ) -> Package {
        let package = Package::build(
            Identifier::Command,
            Instruction::Search,
            Payload::Search(buffer_id, start_page, num),
        );

        package
    }

    /// Fingerprint image collection extension command - GetImageEx(0x28)
    /// Description: Detect the finger, record the fingerprint image and store it in ImageBuffer, return it and record the successful confirmation code; If no finger is detected, return no finger confirmation code (the module responds quickly to each instruction, therefore, for continuous detection, cycle processing is required, which can be limited to the number of cycles or the total time).
    /// Differences between GetImageEx and the GetImage:
    ///     GetImage: return the confirmation code 0x00 when the image quality is too bad (image collection succeeded)
    ///     GetImageEx: return the confirmation code 0x07 when the image quality is too bad (poor collection quality)
    /// Input Parameter: none
    /// Return Parameter: Confirmation code
    ///     0x00: Read success
    ///     0x01: Error when receiving package;
    ///     0x02: No fingers on the sensor;
    ///     0x03: Unsuccessful entry
    ///     0x07: Poor image quality;
    /// Instuction code: 0x28
    pub fn get_image_ex(&mut self) -> Package {
        let package = Package::build(
            Identifier::Command,
            Instruction::GetImageEx,
            Payload::GetImageEx,
        );

        package
    }

    /// Cancel instruction - Cancel(0x30)
    /// Description: Cancel instruction
    /// Input Parameter: none
    /// Return Parameter: Confirmation code
    ///     0x00: Cancel setting successful;
    ///     other: Cancel setting failed;
    /// Instuction code: 0x30
    pub fn cancel(&mut self) -> Package {
        let package = Package::build(Identifier::Command, Instruction::Cancel, Payload::Cancel);

        package
    }

    /// HandShake - HandShake(0x40)
    /// Description: Send handshake instructions to the module. If the module works normally, the confirmation code 0x00 will be returned. The upper computer can continue to send instructions to the module.If the confirmation code is other or no reply, it means that the device is abnormal.
    /// Input Parameter: none
    /// Return Parameter: Confirmation code
    ///     0x00: The device is normal and can receive instructions;
    ///     other: The device is abnormal.
    /// Instuction code: 0x40
    ///     In addition, after the module is powered on, 0x55 will be automatically sent as a handshake sign. After the single-chip microcomputer detects 0x55, it can immediately send commands to enter the working state.
    pub fn handshake(&mut self) -> Package {
        let package = Package::build(
            Identifier::Command,
            Instruction::HandShake,
            Payload::HandShake,
        );

        package
    }

    /// CheckSensor - CheckSensor (0x36)
    /// Description: Check whether the sensor is normal
    /// Input Parameter: none
    /// Return Parameter: Confirmation code
    ///     0x00: The sensor is normal;
    ///     0x29: the sensor is abnormal;
    /// Instuction code: 0x36
    pub fn check_sensor(&mut self) -> Package {
        let package = Package::build(
            Identifier::Command,
            Instruction::CheckSensor,
            Payload::CheckSensor,
        );

        package
    }

    /// Get the algorithm library version - GetAlgVer (0x39)
    /// Description: Get the algorithm library version
    /// Input Parameter: none
    /// Return Parameter: Confirmation code+AlgVer(algorithm library version string)
    ///     0x00: Success;
    ///     0x01: Error when receiving package;
    /// Instuction code: 0x39
    pub fn get_alg_ver(&mut self) -> Package {
        let package = Package::build(
            Identifier::Command,
            Instruction::GetAlgVer,
            Payload::GetAlgVer,
        );

        package
    }

    /// Get the firmware version - GetFwVer (0x3A)
    /// Description: Get the firmware version
    /// Input Parameter: none
    /// Return Parameter: Confirmation code+FwVer(Firmware version string)
    ///     0x00: Success;
    ///     0x01: Error when receiving package;
    /// Instuction code: 0x3A
    pub fn get_fw_ver(&mut self) -> Package {
        let package = Package::build(
            Identifier::Command,
            Instruction::GetFwVer,
            Payload::GetFwVer,
        );

        package
    }

    /// Read product information - ReadProdInfo (0x3C)
    /// Description: Read product information
    /// Input Parameter: none
    /// Return Parameter: Confirmation code+ProdInfo(product information)
    ///     0x00: Success;
    ///     0x01: Error when receiving package;
    /// Instuction code: 0x3C
    pub fn read_prod_info(&mut self) -> Package {
        let package = Package::build(
            Identifier::Command,
            Instruction::ReadProdInfo,
            Payload::ReadProdInfo,
        );

        package
    }

    /// Soft reset SoftRst (0x3D)
    /// Description: Send soft reset instruction to the module. If the module works normally, return confirmation code 0x00, and then perform reset operation.
    /// Input Parameter: none
    /// Return Parameter: Confirmation code
    ///     0x00: Success;
    ///     other: Device is abnormal
    /// Instuction code: 0x3D
    ///     After module reset, 0x55 will be automatically sent as a handshake sign. After the single-chip microcomputer detects 0x55, it can immediately send commands to enter the working state.
    pub fn soft_rst(&mut self) -> Package {
        let package = Package::build(Identifier::Command, Instruction::SoftRst, Payload::SoftRst);

        package
    }

    /// Aura control - AuraLedConfig (0x35)
    /// Description: Aura LED control
    /// Input Parameter: Control code: Ctrl; Speed; ColorIndex; Times
    /// Return Parameter: Confirmation code
    ///     0x00: Success;
    ///     0x01: Error when receiving package;
    /// Instuction code: 0x35
    pub fn aura_led_config(
        &mut self,
        ctrl: LightPattern,
        speed: Speed,
        color: Color,
        count: Times,
    ) -> Package {
        let package = Package::build(
            Identifier::Command,
            Instruction::AuraLedConfig,
            Payload::AuraLedConfig(ctrl, speed, color, count),
        );

        package
    }

    /// Automatic registration template - AutoEnroll (0x31)
    /// Description: When a fingerprint is recorded using an automatic registration template, the fingerprint image needs to be recorded six times for each fingerprint template. The blue light blinks when the fingerprint image is collected. The yellow light is on means the fingerprint image is collected successfully,the green light blinks means the fingerprint characteristic is generated successfully. If the finger is required to leave during image collection, the image will be collected again after the finger is lifted. During the process of waiting for the finger to leave, the white light flashes. After fingerprint images are collected for 6 times and features are generated successfully, features are synthesized and store fingerprint template. If the operation succeeds, the green light is on; if the operation fails, the red light is on. If the finger is away from the sensor for more than 10 seconds when in collecting the fingerprint image each time, it will automatically exits the automatic template registration process.
    /// Input Paramater: ModelID (Fingerprint library location number)
    ///     Config1: Whether to allow cover ID number
    ///         Whether to allow cover ID number: 0: Not allowed 1: Allow
    ///     Config2: Whether to allow duplicate fingerprints
    ///         Whether to allow register duplicate fingerprints: 0: Not allowed 1: Allow
    ///     Config3: Whether the module return the status in the critical step
    ///         Whether to return to the critical step status during registration: 0: Not allowed 1: Allow
    ///     Config4: Whether to allow ask the finger to leave
    ///         Whether the finger is required to leave during the registration process in order to enter the next fingerprint image collection: 0: don't need to leave 1: have to leave
    /// Return Parameter: Confirmation code + ModelID (Fingerprint library location number)
    /// Instruction code: 0x31
    ///     0x00: Set successfully;
    ///     0x01: Set fails;
    ///     0x07: Failed to generate a feature;
    ///     0x0a: Failed to merge templates;
    ///     0x0b: The ID is out of range;
    ///     0x1f: Fingerprint library is full;
    ///     0x22: Fingerprint template is empty;
    ///     0x26: Times out;
    ///     0x27: Fingerprint already exists;
    /// Note: Model ID: Location ID : 0-0xC7, 0xC8-0xFF is automatic filling (The ID number is assigned by the system; The system will be starting from template 0 to searches the empty templates.)
    pub fn auto_enroll(
        &mut self,
        model_id: IndexPage,
        allow_cover_id: bool,
        allow_duplicate: bool,
        return_in_critical: bool,
        ask_finger_to_leave: bool,
    ) -> Package {
        let package = Package::build(
            Identifier::Command,
            Instruction::AutoEnroll,
            Payload::AutoEnroll(
                model_id,
                allow_cover_id,
                allow_duplicate,
                return_in_critical,
                ask_finger_to_leave,
            ),
        );

        package
    }

    /// Automatic fingerprint verification - AutoIdentify (0x32)
    /// Description: When the automatic fingerprint verification command is used to search and verify a fingerprint, the system automatically collects a fingerprint image and generates features, and compares the image with the fingerprint template in the fingerprint database. If the comparison is successful, the system returns the template ID number and the comparison score. If the comparison fails, the system returns the corresponding error code.
    ///              When obtaining the fingerprint image, the fingerprint head will light up with a white breathing light. After the image collection is successful, the yellow light will light up, and the green light will light up after the comparison is successful. If there is a fingerprint image collection error or no fingerprint search, the red light will be on to prompt.
    ///              If the system does not detect the finger for more than 10 seconds after sending the command or collecting the fingerprint image again after reporting an error, it will automatically exit the command.
    /// Input Paramater: SafeGrade (1-5 level)
    ///                  StartPos (0-199)
    ///                  Num - Number of searches (0-199)
    ///                  Config1 - Whether the module returns to the status in key steps
    ///                  Config2 - Number of fingerprint search error
    /// Return Parameter: Confirmation code + ModelID + MarchScore
    ///     0x00: Set successfully;
    ///     0x01: Set fails;
    ///     0x09: Failed to search fingerprint;
    ///     0x0b: The ID is out of range;
    ///     0x22: Fingerprint template is empty;
    ///     0x24: Fingerprint library is empty;
    ///     0x26: Times out;
    /// Instruction code: 0x32
    pub fn auto_identify(
        &mut self,
        grade: SafeGrade,
        start: u8,
        num: u8,
        times: u8,
        return_in_critical: bool,
    ) -> Package {
        let package = Package::build(
            Identifier::Command,
            Instruction::AutoIdentify,
            Payload::AutoIdentify(grade, start, num, times, return_in_critical),
        );

        package
    }

    // # Other instructions

    /// To generate a random code - GetRandomCode
    /// Description: to command the Module to generate a random number and return it to upper computer;Refer to 4.8 for more about Random Number Generator;
    /// Input Parameter: none
    /// Return Parameter: Confirmation code (1 byte)
    ///     0x00: Generation success;
    ///     0x01: Error when receiving package;
    /// Instuction code: 0x14
    pub fn get_random_code(&mut self) -> Package {
        let package = Package::build(
            Identifier::Command,
            Instruction::GetRandomCode,
            Payload::GetRandomCode,
        );

        package
    }

    /// To read information page - ReadInfPage
    /// Description: read information page (512bytes)
    /// Input Parameter: none
    /// Return Parameter: Confirmation code (1 byte)
    ///     0x00: Ready to transfer the following data packet;
    ///     0x01: Error when receiving package;
    ///     0x0F: Can not transfer the following data packet;
    /// Instuction code: 0x16
    /// Note: Module shall transfer following data packet after responding to the upper computer;
    /// Note: Packet Bytes N is determined by Packet Length. The value is 128 Bytes before delivery;
    /// Note: The instruction doesn't affect buffer contents;
    pub fn read_inf_page(&mut self) -> Package {
        let package = Package::build(
            Identifier::Command,
            Instruction::ReadInfPage,
            Payload::ReadInfPage,
        );

        package
    }

    /// To write note pad - WriteNotepad
    /// Description: For upper computer to write data to the specified Flash page; Also see ReadNotepad;
    /// Input Parameter: NotePageNum, user content (or data content)
    /// Return Parameter: Confirmation code (1 byte)
    ///     0x00: Write success;
    ///     0x01: Error when receiving package;
    ///     0x18: Error when write FLASH;
    /// Instuction code: 0x18
    pub fn write_notepad(
        &mut self,
        note_page_number: NotePageNum,
        content: Page,
    ) -> Package {
        let package = Package::build(
            Identifier::Command,
            Instruction::WriteNotepad,
            Payload::WriteNotepad(note_page_number, content),
        );

        package
    }

    /// To read note pad - ReadNotepad
    /// Description: To read the specified page's data content; Also see WriteNotepad.
    /// Input Parameter: NotePageNum
    /// Return Parameter: Confirmation code (1 byte) + data content
    ///     0x00: Read success;
    ///     0x01: Error when receiving package;
    /// Instuction code: 0x19
    pub fn read_notepad(&mut self, note_page_number: NotePageNum) -> Package {
        let package = Package::build(
            Identifier::Command,
            Instruction::ReadNotepad,
            Payload::ReadNotepad(note_page_number),
        );

        package
    }
}

// # Internal Functions

#[inline]
pub fn get_u16_as_u16_parts(value: u16) -> [u16; 2] {
    [value >> 8, value & 0xFF]
}

#[inline]
pub fn get_u32_as_u16_parts(value: u32) -> [u16; 4] {
    [
        (value >> 24 & 0xFF) as u16,
        (value >> 16 & 0xFF) as u16,
        (value >> 8 & 0xFF) as u16,
        (value & 0xFF) as u16,
    ]
}

// # Module Defined Types

#[derive(Default)]
#[allow(dead_code)]
pub struct BasicParameters {
    /// Contents of system status register
    status_register: SystemRegister,
    /// Fixed value: 0x0009
    system_identifier_code: u16,
    /// Finger library size
    finger_libary_size: u16,
    /// Security level (1, 2, 3, 4, 5)
    security_level: u16,
    /// 32-bit device address
    device_address: [u8; 4],
    /// Size code (0, 1, 2, 3)
    data_packet_size: u16,
    /// N (baud = 9600*N bps)
    buad_setting: u16,
}

/// Index tables are read per page, 256 templates per page
/// Index page 0 means to read 0 ~ 255 fingerprint template index table
/// Index page 1 means to read 256 ~ 511 fingerprint template index table
/// Index page 2 means to read 512 ~ 767 fingerprint template index table
/// Index page 3 means to read 768 ~ 1023 fingerprint template index table
pub enum IndexPage {
    Page0 = 0,
    Page1 = 1,
    Page2 = 2,
    Page3 = 3,
}

/// Index table structure: every 8 bits is a group, and each group is output starting from the high position.
/// Data "0" in the index table means that there is no valid template in the corresponding position;
/// Data "1" means that there is a valid template in the corresponding position.
pub type IndexTable = [u8; 32];

#[repr(u8)]
#[derive(Default)]
pub enum BufferID {
    CharBuffer1 = 0x01,
    CharBuffer2 = 0x02,
    CharBuffer3 = 0x03,
    CharBuffer4 = 0x04,
    CharBuffer5 = 0x05,
    #[default]
    CharBuffer6 = 0x06,
}

#[allow(dead_code)]
type MatchScore = u16;

#[allow(dead_code)]
pub struct PageIndex {
    start_page: IndexPage,
    entry: u8,
}

pub type AlgVer = u32;

pub type FwVer = u32;

/// Product information: store in the following order. For Numbers, the high byte comes first. For a string, the insufficient part is 0x00.
#[derive(Default)]
#[allow(dead_code)]
pub struct ProdInfo {
    /// module type, ASCII
    fpm_model: [char; 16],
    /// Module batch number, ASCII
    bn: [char; 4],
    /// Module serial number, ASCII
    sn: [char; 8],
    /// For the hardware version, the first byte represents the main version and the second byte represents the sub-version
    hw_ver: [u8; 2],
    /// Finger Print Sensor Model, ASCII
    fps_model: [char; 8],
    /// Finger Print Sensor Width
    fps_width: u16,
    /// Finger Print Sensor Height
    fps_height: u16,
    /// Template size
    tmpl_size: u16,
    /// Fingerprint database size
    tmpl_total: u16,
}

#[repr(u8)]
#[derive(Clone, PartialEq, Eq)]
pub enum LightPattern {
    Breathing = 0x01,
    Flashing = 0x02,
    AlwaysOn = 0x03,
    AlwaysOff = 0x04,
    GraduallyOn = 0x05,
    GraduallyOff = 0x06,
}

impl From<LightPattern> for u8 {
    fn from(item: LightPattern) -> Self {
        item as u8
    }
}

/// Speed: 0x00-0xff, 256 gears, Minimum 5s cycle.
/// It is effective for breathing lamp and flashing lamp, Light gradually on, Light gradually off
pub type Speed = u8;

#[repr(u8)]
#[derive(Clone, PartialEq, Eq)]
pub enum Color {
    Red = 0b0000_0001,    // 0x01,
    Blue = 0b0000_0010,   // 0x02,
    Purple = 0b0000_0011, // 0x03,
    Green = 0b0000_0100,  // 0x04,
    Yellow = 0b0000_0101, // 0x05,
    Cyan = 0b0000_0110,   // 0x06,
    White = 0b0000_0111,  // 0x07,
}

impl From<Color> for u8 {
    fn from(item: Color) -> Self {
        item as u8
    }
}

/// Number of cycles: 0 = infinite, 1-255.
/// It is effective for with breathing light and flashing light.
type Times = u8;

#[allow(dead_code)]
enum Paramater1 {
    /// 0x01: Collect image for the first time
    CollectFirst = 0x01,
    /// 0x02: Generate Feature for the first time
    FeatureFirst = 0x02,
    /// 0x03: Collect image for the second time
    CollectSecond = 0x03,
    /// 0x04: Generate Feature for the second time
    FeatureSecond = 0x04,
    /// 0x05: Collect image for the third time
    CollectThird = 0x05,
    /// 0x06: Generate Feature for the third time
    FeatureThird = 0x06,
    /// 0x07: Collect image for the fourth time
    CollectFourth = 0x07,
    /// 0x08: Generate Feature for the fourth time
    FeatureFourth = 0x08,
    /// 0x09: Collect image for the fifth time
    CollectFifth = 0x09,
    /// 0x10: Generate Feature for the fifth time
    /// TODO: Double Check this. Migth be 0x0A
    FeatureFifth = 0x10,
    /// 0x11: Collect image for the sixth time
    /// TODO: Double Check this. Migth be 0x0B
    CollectSixth = 0x11,
    /// 0x12: Generate Feature for the sixth time
    /// TODO: Double Check this. Migth be 0x0C
    FeatureSixth = 0x12,
    /// 0x0D: Repeat fingerprint check
    FingerCheck = 0x0D,
    /// 0x0E: Merge feature
    MergeFeatures = 0x0E,
    /// 0x0F: Storage template
    StoreTemplates = 0x0F,
}

pub enum SafeGrade {
    Low = 1,
    LowMid = 2,
    Mid = 3,
    HighMid = 4,
    High = 5,
}
pub type Page = [u8; 512];

/// # Instructions Table
#[repr(u8)]
pub enum Instruction {
    /// 1 = Collect finger image
    GenImg = 0x01,
    /// 2 = To generate character file from image
    GenChar = 0x02,
    /// 3 = Carry out precise matching of two templates;
    Match = 0x03,
    /// 4 = Search the finger library
    Search = 0x04,
    /// 5 = To combine character files and generate template
    RegModel = 0x05,
    /// 6 = To store template;
    Store = 0x06,
    /// 7 = To read/load template
    LoadChar = 0x07,
    /// 8 = To upload template
    UpChar = 0x08,
    /// 9 = To download template
    DownChar = 0x09,
    /// 10 = To upload image
    UpImage = 0x0A,
    /// 11 = To download image
    DownImage = 0x0B,
    /// 12 = To delete tempates
    DeleteChar = 0x0C,
    /// 13 = To empty the library
    Empty = 0x0D,
    /// 14 = To set system Paramete
    SetSysPara = 0x0E,
    /// 15 = To read system Parameter
    ReadSysPara = 0x0F,
    /// 18 = To set password
    SetPwd = 0x12,
    /// 19 = To verify password
    VfyPwd = 0x13,
    /// 20 = To get random code
    GetRandomCode = 0x14,
    /// 21 = To set device address
    SetAdder = 0x15,
    /// 22 = Read information page
    ReadInfPage = 0x16,
    /// 23 = Port control
    Control = 0x17,
    /// 24 = To write note pad
    WriteNotepad = 0x18,
    /// 25 = To read note pad
    ReadNotepad = 0x19,
    /// 29 = To read finger template numbers
    TempleteNum = 0x1D,
    /// 31 = Read fingerprint template index table
    ReadIndexTable = 0x1F,
    /// 40 = Fingerprint image collection extension command
    GetImageEx = 0x28,
    /// 48 = Cancel instruction
    Cancel = 0x30,
    /// 49 = Automatic registration template
    AutoEnroll = 0x31,
    /// 50 = Automatic fingerprint verification
    AutoIdentify = 0x32,
    /// 53 = Aura Control
    AuraLedConfig = 0x35,
    /// 54 = Check Sensor
    CheckSensor = 0x36,
    /// 57 = Get the algorithm library version
    GetAlgVer = 0x39,
    /// 58 = Get the firmware version
    GetFwVer = 0x3A,
    /// 60 = Read product information
    ReadProdInfo = 0x3C,
    /// 61 = Soft reset
    SoftRst = 0x3D,
    /// 64 = Hand Shake
    HandShake = 0x40,
}
impl From<Instruction> for u8 {
    fn from(item: Instruction) -> Self {
        item as u8
    }
}

// Checksum is calculated on 'length (2 bytes) + data (??)'.
pub fn compute_checksum(buf: Vec<u8, 256>) -> u16 {
    let mut checksum = 0u16;

    let check_end = buf.len();
    let checked_bytes = &buf[6..check_end];
    for byte in checked_bytes {
        checksum += (*byte) as u16;
    }
    return checksum;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let vfy_pwd = Payload::VfyPwd(0xFFFFFFFF_u32);
        assert_eq!(vfy_pwd.len(), 4)
    }
}