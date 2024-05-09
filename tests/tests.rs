use r503::*;

#[test]
fn aura_led_ring() {
    let result = aura_led_config(LightPattern::Breathing, 128, Color::Blue, 2);
    assert_eq!(result, ConfirmationCode::Success);
}

#[test]
fn checksum_templete_num() {
    // From the manual, TempleteNum
    let part1: u16 = 0x01; // Package identifier
    let [part2, part3] = get_u16_as_u16_parts(0x0003); // Package length
    let part4: u16 = 0x1D; // Instruction code

    let mut checksum: u16 = 0;
    checksum = checksum.wrapping_add(part1);
    checksum = checksum.wrapping_add(part2);
    checksum = checksum.wrapping_add(part3);
    checksum = checksum.wrapping_add(part4);

    assert_eq!(checksum, 0x0021);
}

#[test]
fn checksum_gen_img() {
    // From the manual, GenImg
    let part1: u16 = 0x01; // Package identifier
    let [part2, part3] = get_u16_as_u16_parts(0x0003); // Package length
    let part4: u16 = 0x01; // Instruction code

    let mut checksum: u16 = 0;
    checksum = checksum.wrapping_add(part1);
    checksum = checksum.wrapping_add(part2);
    checksum = checksum.wrapping_add(part3);
    checksum = checksum.wrapping_add(part4);

    assert_eq!(checksum, 0x0005);
}
