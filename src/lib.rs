#![no_std]

// DOCUMENTATION: R503 https://cdn-shop.adafruit.com/product-files/4651/4651_R503%20fingerprint%20module%20user%20manual.pdf
// DOUCMENTATION: R302 https://file.vishnumaiea.in/ds/module/biometric/R302-Fingerprint-Module-User-Manual.pdf

// # R503 Fingerprint Module User Manual
// # User Manual
// Hangzhou Grow Technology Co., Ltd.
// 2019.6 Ver: 1.1

#[allow(dead_code)]
const READY: u8 = 0x55;

// ## Preface & Declaration
// Thank you for you selection of R503 Fingerprint Identification Module of GROW.
// The Manual is targeted for hardware & software development engineer, covering module function, hardware and software interface etc. To ensure the developing process goes smoothly, it is highly recommended the Manual is read through carefully.
// Because of the products constantly upgraded and improved, module and the manual content may be changed without prior notice. If you want to get the latest information, please visit our company website (www.hzgrow.com).
// We have been trying our best to enssure you the correctness of the Manual. However, if you have any question or find errorst, feel free to contact us or the authorized agent. We would be very grateful.
// The Manual contains proprietary information of Hangzhou Grow Technology Co., Ltd., which shall not be used by or disclosed to third parties without the permission of GROW, nor for any reproduction and alteration of information without any associated warranties, conditions, limitations, or notices.
// No responsibility or liability is assumed by GROW for the application or use, nor for any infringements of patents or other intellectual property rights of third parties that may result from its use.

// # 1. Introduction
// Power DC: 3.3V
// Interface: UART(3.3V TTL logical level)
// Working current (Fingerprint acquisition): 20mA
// Matching Mode: 1:1 and 1:N
// Standby current (finger detection): Typical touch standby voltage: 3.3V Average current: 2uA
// Characteristic value size: 384 bytes
// Baud rate: (9600*N)bps, N=1~6 (default N=6)
// Template size: 768 bytes
// Image acquiring time: <0.2s
// Image resolution: 508dpi
// Sensing Array: 192*192 pixel
// Detection Area Diameter: 15mm
// Storage capacity: 200
// Security level: 5 (1, 2, 3, 4, 5(highest)) FAR <0.001% FRR <1%
// Generate feature point time: < 500ms
// Starting time: <30ms
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
// Inner diameter:23.5mm Height: 19mm

// # Serial Communication
// Connector: MX1.0--6P
// Pin      Color   Name            Description
// 1        Red     Power Supply    DC3.3V
// 2        Black   GND             Signal ground. Connected to power ground.
// 3        Yellow  TXD             Data output. TTL logical level.
// 4        Green   RXD             Data input. TTL logical level.
// 5        Blue    WAKEUP          Finger Detection Signal.
// 6        White   3.3VT           Touch induction power supply, DC3—6V.

// # Hardware connection
// Via serial interface, the Module may communicate with MCU of 3.3V or 5V power:
// TD (pin 3 of P1) connects with RXD (receiving pin of MCU),
// RD (pin 4 of P1) connects with TXD (transferring pin of MCU).
// Should the upper computer (PC) be in RS-232 mode, please add level converting circuit, like MAX232, between the Module and PC.

// # Serial communication protocol
// The mode is semiduplex asychronism serial communication. And the default baud rate is 57600bps. User may set the baud rate in 9600~115200bps.
// Transferring frame format is 10 bit: the low-level starting bit, 8-bit data with the LSB first, and an ending bit. There is no check bit.

// # Reset time
// At power on, it takes about 200ms for initialization. During this period, the Module can’t accept commands for upper computer.

// # 3. System Resources
// To address demands of different customer, Module system provides abundant resources at user’s use.
// ## Notepad
// The system sets aside a 512-bytes memory (16 pages* 32 bytes) for user’s notepad, where data requiring power-off protection can be stored. The host can access the page by instructions of PS_WriteNotepad and PS_Read Notepad.
// Note: when write on one page of the pad, the entire 32 bytes will be written in wholly covering the original contents.
// ## Buffer
// The module RAM resources are as follows:
// An ImageBuffer: ImageBuffer
// 6 feature buffers: CharBuffer[1:6]
// All buffer contents are not saved without power.
// ## Image buffer
// ImageBuffer serves for image storage and the image format is192*192 pixels.
// When transferring through UART, to quicken speed, only the upper 4 bits of the pixel is transferred (that is 16 grey degrees).
// And two adjacent pixels of the same row will form a byte before the transferring.
// When uploaded to PC, the 16-grey-degree image will be extended to 256-grey-degree format. That’s 8-bit BMP format. TODO: (Is this true in the R503? I Don't know yet.)
// When transferring through USB, the image is 8-bit pixel, that’s 256 grey degrees.
// # Fingerprint Library
// System sets aside a certain space within Flash for fingerprint template storage, that’s fingerprint library. Contents of the library remain at power off.
// Capacity of the library changes with the capacity of Flash, system will recognize the latter automatically. Fingerprint template’s storage in Flash is in sequential order. Assume the fingerprint capacity N, then the serial number of template in library is 0, 1, 2, 3 ... N. User can only access library by template number.
#[repr(u8)]
pub enum ParameterSetting {
    /// The Parameter controls the UART communication speed of the Modul. Its value is an integer N, N= [1/2/4/6/12]. Cooresponding baud rate is 9600*N bps。
    BaudRateControl = 4,
    /// The Parameter controls the matching threshold value of fingerprint searching and matching. Security level is divided into 5 grades, and cooresponding value is 1, 2, 3, 4, 5. At level 1, FAR is the highest and FRR is the lowest; however at level 5, FAR is the lowest and FRR is the highest.
    SecurityLevel = 5,
    /// The parameter decides the max length of the transferring data package when communicating with upper computer. Its value is 0, 1, 2, 3, corresponding to 32 bytes, 64 bytes, 128 bytes, 256 bytes respectively.
    DataPackageLength = 6,
}

/// # Baud rate control (Parameter Number: 4)
/// The Parameter controls the UART communication speed of the Modul. Its value is an integer N, N= [1/2/4/6/12]. Cooresponding baud rate is 9600*N bps.
#[repr(u8)]
pub enum BaudRate {
    Rate9600 = 1,
    Rate19200 = 2,
    Rate38400 = 4,
    Rate57600 = 6,
    Rate115200 = 12,
}

/// # Security Level (Parameter Number: 5)
/// The Parameter controls the matching threshold value of fingerprint searching and matching. Security level is divided into 5 grades, and cooresponding value is 1, 2, 3, 4, 5. At level 1, FAR is the highest and FRR is the lowest; however at level 5, FAR is the lowest and FRR is the highest.
#[repr(u8)]
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
pub enum PacketLength {
    Bytes32 = 0,
    Bytes64 = 1,
    Bytes128 = 2,
    Bytes256 = 3,
}

// # System status register
// System status register indicates the current operation status of the Module. Its length is 1 word, and can be read via instruction ReadSysPara. Definition of the register is as follows:
// Bit Number   Description Notes
// 15  - 4      Reserved    Reserved
// 3            ImgBufStat  1 = Image buffer contains valid image.
// 2            PWD         1 = Verified device’s handshaking password.
// 1            Pass        1 = Find the matching finger; 0 = wrong finger;
// 0            Busy        1 = Sstem is executing commands; 0 = system is free;

#[repr(u16)]
#[derive(Default)]
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
/// The default password of the module is `0x00000000``. If the default password is modified, the first instruction of the upper computer to communicate with the module must be verify password. Only after the password verification is passed, the module will enter the normal working state and receive other instructions.
/// The new modified password is stored in Flash and remains at power off.(the modified password cannot be obtained through the communication instruction. If forgotten by mistake, the module cannot communicate, please use with caution)
/// Refer to instruction SetPwd and VfyPwd.
const PASSWORD: u32 = 0x00000000;

/// ## Module address
/// Each module has an identifying address. When communicating with upper computer, each instruction/data is transferred in data package form, which contains the address item. Module system only responds to data package whose address item value is the same with its identifying address.
/// The address length is 4 bytes, and its default factory value is 0xFFFFFFFF. User may modify the address via instruction SetAdder. The new modified address remains at power off.
const ADDRESS: u32 = 0xFFFFFFFF;

// ## Random number generator
// Module integrates a hardware 32-bit random number generator (RNG) (without seed). Via instruction GetRandomCode, system will generate a random number and upload it.
// # Features and templates
// The chip has an image buffer and six feature file buffers, all buffer contents are not saved after power failure.
// A template can be composed of 2-6 feature files,the more characteristic files the composite template has, the better the quality of the fingerprint template,
// At least 3 template synthesis features are recommended.

// # 4 Communication Protocol
// The protocol defines the data exchanging format when R503 series communicates with upper computer. The protocol and instruction sets apples for both UART and USB communication mode. For PC, USB interface is strongly recommended to improve the exchanging speed, especially in fingerprint scanning device.
// ## 4.1 Data package format
// When communicating, the transferring and receiving of command/data/result are all wrapped in data package format.
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
const HEADER: u16 = 0xEF01;
// Name: Adder
// Symbol: ADDER
// Length: 2 Bytes
// Description: Default value is 0xFFFFFFFF, which can be modified by command. High byte transferred first and at wrong adder value, module will reject to transfer.

// Name: Package identifier
// Symbol: PID
// Length: 1 Byte
// Description:
//      0x01: Command packet;
//      0x02: Data packet; Data packet shall not appear alone in executing processs, must follow command packet or acknowledge packet.
//      0x07: Acknowledge packet;
//      0x08: End of Data packet.
#[repr(u8)]
#[derive(Clone, Copy)]
pub enum Identifier {
    Command = 0x01,
    Data = 0x02,
    Acknowledge = 0x03,
    End = 0x08,
}

/// Name: Package length
/// Symbol: LENGTH
/// Length: 2 Bytes
/// Description: Refers to the length of package content (command packets and data packets) plus the length of Checksum( 2 bytes). Unit is byte. Max length is 256 bytes. And high byte is transferred first.
pub type Length = u16;

// Name: Package contents
// Symbol: DATA
// Length: -
// Description: It can be commands, data, command's parameters, acknowledge result, etc. (fingerprint character value, template are all deemed as data);
pub struct Payload {
    /// The first and maybe only thing is the instruction
    pub instruction: Instruction,
}

impl Default for Payload {
    fn default() -> Self {
        Self {
            instruction: Instruction::SoftRst,
        }
    }
}

// Name: Checksum
// Symbol: SUM
// Length: 2 Bytes
// Description: The arithmetic sum of package identifier, package length and all package contens. Overflowing bits are omitted. high byte is transferred first.
pub type Sum = u16;

pub struct Package {
    /// See Name: Header
    header: u16,
    /// See Name: Adder
    address: u32,
    /// See Name: Package identifier
    identifier: Identifier,
    /// See Name: Package length
    length: Length,
    /// See Name: Package contents
    contents: Payload,
    /// See Name: Checksum
    checksum: Sum,
}

impl Package {
    pub fn get_header(&self) -> u16 {
        self.header
    }
    pub fn get_address(&self) -> u32 {
        self.address
    }
    pub fn set_address(&mut self, address: u32) -> &Self {
        self.address = address;
        self
    }
    pub fn get_identifier(&self) -> Identifier {
        self.identifier
    }
    pub fn set_identifier(&mut self, identifier: Identifier) -> &Self {
        self.identifier = identifier;
        self
    }
    pub fn get_length(&self) -> Length {
        self.length
    }
    pub fn set_length(&mut self, len: Length) {
        self.length = len;
    }
    pub fn get_contents(&self) -> &Payload {
        &self.contents
    }
    pub fn set_contents(&mut self, contents: Payload) {
        self.contents = contents;
    }
    pub fn get_checksum(&self) -> &Sum {
        &self.checksum
    }
    pub fn set_checksum(&mut self, sum: Sum) {
        self.checksum = sum;
    }
    pub fn calc_checksum(&mut self) -> &Sum {
        self.checksum = 0;

        // PID
        self.checksum = self.checksum.wrapping_add(self.get_identifier() as u16);

        // Length
        for byte in get_u16_as_u16_parts(self.get_length()) {
            self.checksum = self.checksum.wrapping_add(byte)
        }

        // Contents

        &self.checksum
    }
}

impl Default for Package {
    fn default() -> Self {
        Self {
            header: HEADER,
            address: ADDRESS,
            identifier: Identifier::Acknowledge,
            length: 12,
            contents: Payload::default(),
            checksum: Sum::default(),
        }
    }
}

// # Check and acknowledgement of data package
// Note: Commands shall only be sent from upper computer to the Module, and the Module acknowledges the commands.
// Upon receipt of commands, Module will report the commands execution status and results to upper computer through acknowledge packet. Acknowledge packet has parameters and may also have following data packet. Upper computer can’t ascertain Module’s package receiving status or command execution results unless through acknowledge packet sent from Module. Acknowledge packet includes 1 byte confirmation code and maybe also the returned parameter.
// Confirmation code’s definition is:

#[repr(u8)]
#[derive(Debug, Default, PartialEq)]
pub enum ConfirmationCode {
    /// Commad Execution Complete;
    Success = 0x00,
    /// Error When Receiving Data Package;
    ErrorReceivingPacket = 0x01,
    /// No Finger on the Sensor;
    NoFingerOnSensor = 0x02,
    /// Fail to Enroll the Finger;
    FailToEnrollFinger = 0x03,
    /// Fail to Generate Character File Due to the Over-disorderly Fingerprint Image;
    FailToGenerateCharacterOverDisorderlyFingerprintImage = 0x06,
    /// Fail to Generate Character File due to Lackness of Character Point or Over-smallness of Fingerprint Image;
    FailToGenerateCharacterLacknessOfCharacterPointOrOverSmallness = 0x07,
    /// Finger Doesn’t Match;
    FailFingerDoesntMatch = 0x08,
    /// Fail to Find the Matching Finger;
    FailToFindMatchingFinger = 0x09,
    /// Fail to Combine the Character Files;
    FailToCombineCharacterFiles = 0x0A,
    /// Addressing PageID is Beyond the Finger Library;
    AddressingPageIDIsBeyoundTheFingerLibary = 0x0B,
    /// Error When Reading Template from Library or the Template is Invalid;
    ErrorWhenReadingTemplateFromLibararORTemplateIsInvalid = 0x0C,
    /// Error When Uploading Template;
    ErrorWhenUploadingTemplate = 0x0D,
    /// Module can’t receive the following data packages;
    ModuleCantReceivingTheFollowingDataPackages = 0x0E,
    /// Error when uploading image;
    ErrorWhenUploadingImage = 0x0F,
    /// Fail to delete the template;
    FailToDeleteTheTemplate = 0x10,
    /// Fail to clear finger library;
    FailToClearFingerLibary = 0x11,
    /// Wrong password;
    WrongPassword = 0x13,
    /// Fail to generate the image for the lackness of valid primary image;
    FailToGenerateImageLacknessOfValidPrimaryImage = 0x15,
    /// Error when writing flash;
    ErrorWhenWritingFlash = 0x18,
    /// No definition error;
    NoDefinitionError = 0x19,
    /// Invalid register number;
    InvalidRegisterNumber = 0x1A,
    /// Incorrect configuration of register;
    IncorrectConfigurationOfRegister = 0x1B,
    /// Wrong notepad page number;
    WrongNotepadPageNumber = 0x1C,
    /// Fail to operate the communication port;
    FailToOperateTheCommunicationPort = 0x1D,
    /// Others: System Reserved; (And Default for this Rust Lib);
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
            0x18 /*  23 */ => Self::ErrorWhenWritingFlash,
            0x19 /*  24 */ => Self::NoDefinitionError,
            0x1A /*  25 */ => Self::InvalidRegisterNumber,
            0x1B /*  26 */ => Self::IncorrectConfigurationOfRegister,
            0x1C /*  27 */ => Self::WrongNotepadPageNumber,
            0x1D /*  28 */ => Self::FailToOperateTheCommunicationPort,
            _ => Self::SystemReserved,
        }
    }
}

// # 5. Module Instruction System
// R30X series provide 23 instructions. R50X series provide 33 instructions. Through combination of different instructions, application program may realize muti finger authentication functions. All commands/data are transferred in package format. Refer to 5.1 for the detailed information of package.

// # System-related instructions

/// Verify passwoard - VfyPwd
/// Description: Verify Module’s handshaking password. (Refer to 4.6 for details)
/// Input Parameter: PassWord (4 bytes)
/// Return Parameter: Confirmation code (1 byte)
///     0x00: Correct password;
///     0x01: Error when receiving package;
///     0x13: Wrong password;
/// Instruction code: 0x13
pub fn vfy_pwd(_password: u32) -> ConfirmationCode {
    let _pw = PASSWORD;
    todo!()
}

/// Set password - SetPwd
/// Description: Set Module’s handshaking password. (Refer to 4.6 for details) Input Parameter: PassWord (4 bytes)
/// Return Parameter: Confirmation code (1 byte)
///     0x00: Password setting complete;
///     0x01: Error when receiving package;
/// Instruction code: 0x12
pub fn set_pwd(_password: [u8; 4]) -> ConfirmationCode {
    todo!()
}

/// Set Module address - SetAdder
/// Description: Set Module address. (Refer to 4.7 for adderss information) Input Parameter: None;
/// Return Parameter: Confirmation code (1 byte)
///     0x00: Address setting complete;
///     0x01: Error when receiving package;
/// Instruction code: 0x15
pub fn set_adder(_address: [u8; 4]) -> ConfirmationCode {
    todo!()
}

/// Set module system’s basic parameter - SetSysPara
/// Description: Operation parameter settings. (Refer to 4.4 for more information) Input Parameter: Parameter number;
/// Return Parameter: Confirmation code (1 byte)
///     0x00: Parameter setting complete;
///     0x01: Error when receiving package;
///     0x1A: Wrong register number;
/// Instruction code: 0x0E
pub fn set_sys_para(_parameter: ParameterSetting) -> ConfirmationCode {
    todo!()
}

#[repr(u8)]
#[derive(Default)]
pub enum PortControl {
    #[default]
    Off = 0,
    On = 1,
}

/// Port Control - Control
/// Description:
///     For UART protocol, it control the "on/off" of USB port;
///     For USB protocol, it control the "on/off" of UART port;
/// Input Parameter: control code
///     Control code ”0” means turns off the port;
///     Control code ”1” means turns on the port;
/// Return Parameter: confirmation code;
///     0x00: Port operation complete;
///     0x01: Error when receiving package;
///     0x1D: Fail to operate the communication port;
/// Instruction code: 0x17
pub fn control(_control_code: PortControl) -> ConfirmationCode {
    ConfirmationCode::Success
}

#[derive(Default)]
#[allow(dead_code)]
pub struct BasicParameters {
    /// Contents of system status register
    status_register: u16,
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

/// Read system Parameter - ReadSysPara
/// Description: Read Module’s status register and system basic configuration parameters(; Refer to 4.4 for system configuration parameter and 4.5 for system status register).
/// Input Parameter:none
/// Return Parameter:Confirmation code (1 byte) + basic parameter (16 bytes)
///     0x00: Read complete;
///     0x01: Error when receiving package;
/// Instuction code: 0x0F
pub fn read_sys_para() -> (ConfirmationCode, BasicParameters) {
    todo!()
}

/// Read valid template number - TempleteNum
/// Description: read the current valid template number of the Module
/// Input Parameter: none
/// Return Parameter: Confirmation code (1 byte)，template number:N
///     0x00: Read success;
///     0x01: Error when receiving package;
/// Instuction code: 0x1D
pub fn templete_num() -> (ConfirmationCode, u8) {
    todo!()
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

/// Read fingerprint template index table - ReadIndexTable(0x1F)
/// Description: Read the fingerprint template index table of the module, read the index table of the fingerprint template up to 256 at a time (32 bytes)
/// Input Parameter: Index page
/// Return Parameter: Confirmation code+Fingerprint template index table
///     0x00: Read complete;
///     0x01: Error when receiving package;
/// Instuction code: 0x1F
pub fn read_index_table(_index_page: IndexPage) -> (ConfirmationCode, IndexTable) {
    todo!()
}

// # Fingerprint-processing instructions

/// To collect finger image - GenImg
/// Description: detecting finger and store the detected finger image in ImageBuffer while returning successfull confirmation code; If there is no finger, returned confirmation code would be “can’t detect finger”.
/// Input Parameter: none
/// Return Parameter: Confirmation code (1 byte)
///     0x00: Finger collection successs;
///     0x01: Error when receiving package;
///     0x02: Can’t detect finger;
///     0x03: Fail to collect finger;
/// Instuction code: 0x01
pub fn gen_img() -> ConfirmationCode {
    todo!()
}

/// TODO: Make this actually real.
/// Template size: 24768 bytes.
/// 192px * 192px Monochrome
type ImageData = [u8; 24768];

/// TODO: Make this actually real.
/// Template size: 768 bytes.
#[allow(dead_code)]
type TemplateData = [u8; 768];

/// Upload image - UpImage
/// Description: to upload the image in Img_Buffer to upper computer. Refer to 1.1.1 for more about image buffer.
/// Input Parameter: none
/// Return Parameter: Confirmation code (1 byte)
///     0x00: Ready to transfer the following data packet;
///     0x01: Error when receiving package;
///     0x0F: Fail to transfer the following data packet;
/// Instuction code: 0x0A
///     Module shall transfer the following data packet after responding to the upper computer.
pub fn up_image() -> (ConfirmationCode, ImageData) {
    todo!()
}

/// Download the image - DownImage
/// Description: to download image from upper computer to Img_Buffer. Refer to 1.1.1 for more about the image buffer.
/// Input Parameter: none
/// Return Parameter: Confirmation code (1 byte)
///     0x00: Ready to transfer the following data packet;
///     0x01: Error when receiving package;
///     0x0E: Fail to transfer the following data packet;
/// Instuction code: 0x0B
///     Module shall transfer the following data packet after responding to the upper computer. Data package length must be 64, 128, or 256.
pub fn down_image(_image: ImageData) -> ConfirmationCode {
    todo!()
}

/// TODO: Make this actually real.
/// Characteristic value size: 384 bytes.
type CharacterData = [u8; 384];

#[repr(u8)]
#[derive(Default)]
pub enum BufferID {
    CharBuffer1 = 0x01,
    #[default]
    CharBuffer2 = 0x02,
}

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
pub fn img2_tz(_buffer_id: BufferID) -> ConfirmationCode {
    todo!()
}

/// To generate template - RegModel
/// Description: To combine information of character files from CharBuffer1 and CharBuffer2 and generate a template which is stroed back in both CharBuffer1 and CharBuffer2.
/// Input Parameter: none
/// Return Parameter:Confirmation code (1 byte)
///     0x00: Operation success;
///     0x01: Error when receiving package;
///     0x0A: Fail to combine the character files. That’s, the character files don’t belong to one finger.
/// Instuction code: 0x05
pub fn reg_model() -> ConfirmationCode {
    todo!()
}

/// To upload character or template - UpChar
/// Description: to upload the character file or template of CharBuffer1/CharBuffer2 to upper computer;
/// Input Parameter: BufferID (Buffer number)
/// Return Parameter: Confirmation code (1 byte)
///     0x00: Ready to transfer the following data packet;
///     0x01: Error when receiving package;
///     0x0D: Eerror when uploading template;
/// Instuction code: 0x08
pub fn up_char(_buffer_id: BufferID) -> ConfirmationCode {
    todo!()
}

/// Download template - DownChar
/// Description: upper computer download template to module buffer
/// Input Parameter: CharBufferID (Buffer number)
/// Return Parameter: Confirmation code (1 byte)
///     0x00: Ready to transfer the following data packet;
///     0x01: Error when receiving package;
///     0x0E: Can not receive the following data packet
/// Instuction code: 0x09
///     Module shall transfer following data packet after responding to the upper computer.;
///     The instruction doesn’t affect buffer contents.
pub fn down_char(_buffer_id: BufferID, _template: CharacterData) -> ConfirmationCode {
    todo!()
}

/// To store template - Store
/// Description: to store the template of specified buffer (Buffer1/Buffer2) at the designated location of Flash library.
/// Input Parameter: BufferID(buffer number), PageID(Flash location of the template, two bytes with high byte front and low byte behind)
/// Return Parameter: Confirmation code (1 byte)
///     0x00: Storage success;
///     0x01: Error when receiving package;
///     0x0B: Addressing PageID is beyond the finger library;
///     0x18: Error when writing Flash.
/// Instuction code: 0x06
pub fn store(_buffer_id: BufferID, _page_id: IndexPage) -> ConfirmationCode {
    // TODO: This one is a little funky. Page_ID param expects a [u8; 2].
    // The first byte being the page number, (0-3)
    // The second byte being the index in that page. (0-255)
    todo!()
}

/// To read template from Flash library - LoadChar
/// Description: to load template at the specified location (PageID) of Flash library to template buffer CharBuffer1/CharBuffer2
/// Input Parameter: BufferID(buffer number), PageID (Flash location of the template, two bytes with high byte front and low byte behind)。
/// Return Parameter: Confirmation code (1 byte)
///     0x00: Load success;
///     0x01: Error when receiving package;
///     0x0C: Error when reading template from library or the readout template is invalid;
///     0x0B: Addressing PageID is beyond the finger library;
/// Instuction code: 07H
pub fn load_char(_buffer_id: BufferID, _page_id: IndexPage) -> ConfirmationCode {
    // TODO: This one is a little funky. Page_ID param expects a [u8; 2].
    // The first byte being the page number, (0-3)
    // The second byte being the index in that page. (0-255)
    todo!()
}

/// To delete template - DeletChar
/// Description: to delete a segment (N) of templates of Flash library started from the specified location (or PageID);
/// Input Parameter: PageID (template number in Flash), N (number of templates to be deleted)
/// Return Parameter: Confirmation code (1 byte)
///     0x00: Delete success;
///     0x01: Error when receiving package;
///     0x10: Failed to delete templates;
/// Instuction code: 0x0C
pub fn delete_char(_buffer_id: BufferID, _n: u8) -> ConfirmationCode {
    // TODO: This one is a little funky. Page_ID param expects a [u8; 2].
    // The first byte being the page number, (0-3)
    // The second byte being the index in that page. (0-255)
    todo!()
}

/// To empty finger library - Empty
/// Description: to delete all the templates in the Flash library
/// Input Parameter: none
/// Return Parameter: Confirmation code (1 byte)
///     0x00: Empty success;
///     0x01: Error when receiving package;
///     0x11: Fail to clear finger library;
/// Instuction code: 0x0D
pub fn empty() -> ConfirmationCode {
    todo!()
}

/// To carry out precise matching of two finger templates - Match
/// Description: to carry out precise matching of templates from CharBuffer1 and CharBuffer2, providing matching results.
/// Input Parameter: none
/// Return Parameter: Confirmation code (1 byte)，matching score.
///     0x00: templates of the two buffers are matching!
///     0x01: error when receiving package;
///     0x08: templates of the two buffers aren’t matching;
/// Instuction code: 0x03
///     The instruction doesn’t affect the contents of the buffers.
pub fn r#match() -> (ConfirmationCode, u8) {
    todo!()
}

#[allow(dead_code)]
pub struct PageIndex {
    start_page: IndexPage,
    entry: u8,
}

/// To search finger library Search
/// Description: to search the whole finger library for the template that matches the one in CharBuffer1 or CharBuffer2. When found, PageID will be returned.
/// Input Parameter: BufferID, StartPage (searching start address), PageNum(searching numbers)
/// Return Parameter: Confirmation code (1 byte), PageID (matching templates location)
///     0x00: found the matching finer;
///     0x01: error when receiving package;
///     0x09: No matching in the library (both the PageID and matching score are 0);
/// Instuction code: 0x04
///     The instruction doesn’t affect the contents of the buffers.
pub fn search(
    _buffer_id: BufferID,
    _start_page: u8,
    _page_num: u8,
) -> (ConfirmationCode, PageIndex) {
    todo!()
}

/// Fingerprint image collection extension command - GetImageEx(0x28)
/// Description: Detect the finger, record the fingerprint image and store it in ImageBuffer, return it and record the successful confirmation code; If no finger is detected, return no finger confirmation code(the module responds quickly to each instruction,therefore, for continuous detection, cycle processing is required, which can be limited to the number of cycles or the total time).
/// Differences between GetImageEx and the GetImage:
/// GetImage: return the confirmation code 0x00 when the image quality is too bad (image collection succeeded)
/// GetImageEx: return the confirmation code 0x07 when the image quality is too bad (poor collection quality)
/// Input Parameter: none
/// Return Parameter: Confirmation code
///     0x00: Read success
///     0x01: Error when receiving package;
///     0x02: No fingers on the sensor;
///     0x03: Unsuccessful entry
///     0x07: Poor image quality;
/// Instuction code: 0x28
pub fn get_image_ex() -> ConfirmationCode {
    todo!()
}

/// Cancel instruction - Cancel(0x30)
/// Description: Cancel instruction
/// Input Parameter: none
/// Return Parameter: Confirmation code
///     0x00: Cancel setting successful;
///     other: Cancel setting failed;
/// Instuction code: 0x30
pub fn cancel() -> ConfirmationCode {
    todo!()
}

/// HandShake - HandShake(0x40)
/// Description: Send handshake instructions to the module. If the module works normally, the confirmation code 0x00 will be returned. The upper computer can continue to send instructions to the module.If the confirmation code is other or no reply, it means that the device is abnormal.
/// Input Parameter: none
/// Return Parameter: Confirmation code
///     0x00: The device is normal and can receive instructions;
///     other: The device is abnormal.
/// Instuction code: 0x40
///     In addition, after the module is powered on, 0x55 will be automatically sent as a handshake sign. After the single-chip microcomputer detects 0x55, it can immediately send commands to enter the working state.
pub fn handshake() -> ConfirmationCode {
    todo!()
}

/// CheckSensor - CheckSensor (0x36)
/// Description: Check whether the sensor is normal
/// Input Parameter: none
/// Return Parameter: Confirmation code
///     0x00: The sensor is normal;
///     0x29: the sensor is abnormal;
/// Instuction code: 0x36
pub fn check_sensor() -> ConfirmationCode {
    todo!()
}

pub type AlgVer = u32;

/// Get the algorithm library version - GetAlgVer (0x39)
/// Description: Get the algorithm library version
/// Input Parameter: none
/// Return Parameter: Confirmation code+AlgVer(algorithm library version string)
///     0x00: Success;
///     0x01: Error when receiving package;
/// Instuction code: 0x39
pub fn get_alg_ver() -> (ConfirmationCode, AlgVer) {
    todo!()
}

pub type FwVer = u32;

/// Get the firmware version - GetFwVer (0x3A)
/// Description: Get the firmware version
/// Input Parameter: none
/// Return Parameter: Confirmation code+FwVer(Firmware version string)
///     0x00: Success;
///     0x01: Error when receiving package;
/// Instuction code: 0x3A
pub fn get_fw_ver() -> (ConfirmationCode, FwVer) {
    todo!()
}

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

/// Read product information - ReadProdInfo (0x3C)
/// Description: Read product information
/// Input Parameter: none
/// Return Parameter: Confirmation code+ProdInfo(product information)
///     0x00: Success;
///     0x01: Error when receiving package;
/// Instuction code: 0x3C
pub fn read_prod_info() -> (ConfirmationCode, ProdInfo) {
    todo!()
}

/// Soft reset SoftRst (0x3D)
/// Description: Send soft reset instruction to the module. If the module works normally, return confirmation code 0x00, and then perform reset operation.
/// Input Parameter: none
/// Return Parameter: Confirmation code
///     0x00: Success;
///     other: Device is abnormal
/// Instuction code: 0x3D
///     After module reset, 0x55 will be automatically sent as a handshake sign. After the single-chip microcomputer detects 0x55, it can immediately send commands to enter the working state.
pub fn soft_rst() -> ConfirmationCode {
    todo!()
}

#[repr(u8)]
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
pub enum ColorIndex {
    Red = 0b0000_0001,    // 0x01,
    Blue = 0b0000_0010,   // 0x02,
    Purple = 0b0000_0011, // 0x03,
    Green = 0b0000_0100,  // 0x04,
    Yellow = 0b0000_0101, // 0x05,
    Cyan = 0b0000_0110,   // 0x06,
    White = 0b0000_0111,  // 0x07,
}

impl From<ColorIndex> for u8 {
    fn from(item: ColorIndex) -> Self {
        item as u8
    }
}

/// Number of cycles: 0 = infinite, 1-255.
/// It is effective for with breathing light and flashing light.
type Times = u8;

/// Aura control - AuraLedConfig (0x35)
/// Description: Aura LED control
/// Input Parameter: Control code: Ctrl; Speed; ColorIndex; Times
/// Return Parameter: Confirmation code
///     0x00: Success;
///     0x01: Error when receiving package;
/// Instuction code: 0x35
pub fn aura_led_config(
    ctrl: LightPattern,
    speed: Speed,
    color_index: ColorIndex,
    count: Times,
) -> ConfirmationCode {
    let mut packet: [u8; 16] = [0x00; 16];
    packet[10] = ctrl.into();
    packet[11] = speed;
    packet[12] = color_index.into();
    packet[13] = count;
    todo!()
}

// # Other instructions

/// To generate a random code - GetRandomCode
/// Description: to command the Module to generate a random number and return it to upper computer;Refer to 4.8 for more about Random Number Generator;
/// Input Parameter: none
/// Return Parameter: Confirmation code (1 byte)
///     0x00: Generation success;
///     0x01: Error when receiving package;
/// Instuction code: 0x14
pub fn get_random_code() -> ConfirmationCode {
    let _packet: Package = Package {
        identifier: Identifier::Command,
        length: 4,
        contents: Payload {
            instruction: Instruction::GetRandomCode,
        },
        checksum: Sum::default(),
        ..Default::default()
    };

    ConfirmationCode::Success
}

pub type Page = [u8; 512];

/// To read information page - ReadInfPage
/// Description: read information page (512bytes)
/// Input Parameter: none
/// Return Parameter: Confirmation code (1 byte)
///     0x00: Ready to transfer the following data packet;
///     0x01: Error when receiving package;
///     0x0F: Can not transfer the following data packet;
/// Instuction code: 0x16
///     Module shall transfer following data packet after responding to the upper computer.;
///     The instruction doesn’t affect buffer contents.
pub fn read_inf_page() -> (ConfirmationCode, Page) {
    todo!()
}

/// To write note pad - WriteNotepad
/// Description: for upper computer to write data to the specified Flash page (refer to 4.1 for more about Note pad). Also see ReadNotepad;
/// Input Parameter: NotePageNum, user content (or data content)
/// Return Parameter: Confirmation code (1 byte)
///     0x00: Write success;
///     0x01: Error when receiving package;
/// Instuction code: 0x18
pub fn write_notepad(_note_page_number: u8, _content: Page) {
    todo!()
}

/// To read note pad - ReadNotepad
/// Description: to read the specified page’s data content; Refer to 4.1 for more about user note pad. Also see WriteNotepad.
/// Input Parameter: none
/// Return Parameter: Confirmation code (1 byte) + data content
///     0x00: Read success;
///     0x01: Error when receiving package;
/// Instuction code: 0x19
pub fn read_notepad(_note_page_number: u8, _content: Page) {
    todo!()
}

/// # Instructions Table
#[repr(u8)]
pub enum Instruction {
    /// Collect finger image
    GenImg = 0x01,
    /// To generate character file from image
    Img2Tz = 0x02,
    /// Carry out precise matching of two templates;
    Match = 0x03,
    /// Search the finger library
    Search = 0x04,
    /// To combine character files and generate template
    RegModel = 0x05,
    /// To store template;
    Store = 0x06,
    /// To read/load template
    LoadChar = 0x07,
    /// To upload template
    UpChar = 0x08,
    /// To download template
    DownChar = 0x09,
    /// To upload image
    UpImage = 0x0A,
    /// To download image
    DownImage = 0x0B,
    /// To delete tempates
    DeleteChar = 0x0C,
    /// To empty the library
    Empty = 0x0D,
    /// To set system Paramete
    SetSysPara = 0x0E,
    /// To read system Parameter
    ReadSysPara = 0x0F,
    /// To set password
    SetPwd = 0x12,
    /// To verify password
    VfyPwd = 0x13,
    /// To get random code
    GetRandomCode = 0x14,
    /// To set device address
    SetAdder = 0x15,
    /// Read information page
    ReadInfPage = 0x16,
    /// Port control
    Control = 0x17,
    /// To write note pad
    WriteNotepad = 0x18,
    /// To read note pad
    ReadNotepad = 0x19,
    /// To read finger template numbers
    TempleteNum = 0x1D,
    /// Read fingerprint template index table
    ReadIndexTable = 0x1F,
    /// Fingerprint image collection extension command
    GetImageEx = 0x28,
    /// Cancel instruction
    Cancel = 0x30,
    /// Aura Control
    AuraLedConfig = 0x35,
    /// Check Sensor
    CheckSensor = 0x36,
    /// Get the algorithm library version
    GetAlgVer = 0x39,
    /// Get the firmware version
    GetFwVer = 0x3A,
    /// Soft reset
    SoftRst = 0x3D,
    /// Hand Shake
    HandShake = 0x40,
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
