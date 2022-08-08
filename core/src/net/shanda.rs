use bytes::BytesMut;

pub fn encrypt(mut data: BytesMut) -> BytesMut {
    let length = data.len();

    for _ in 0..3 {
        let mut a = 0;

        for j in (1..=length).rev() {
            let mut c = i32::from(data[length - j]);
            c = roll_left(c, 3);
            c += j as i32;
            c &= 0xff;
            c ^= a;
            a = c;
            c = roll_right(a, j as i32);
            c ^= 0xff;
            c += 0x48;
            c &= 0xff;
            data[length - j] = c as u8;
        }

        a = 0;

        for j in (1..=length).rev() {
            let mut c = i32::from(data[j - 1]);
            c = roll_left(c, 4);
            c += j as i32;
            c &= 0xff;
            c ^= a;
            a = c;
            c ^= 0x13;
            c = roll_right(c, 3);
            data[j - 1] = c as u8;
        }
    }

    data
}

pub fn decrypt(mut data: BytesMut) -> BytesMut {
    let length = data.len();

    for _ in 0..3 {
        let mut a: i32;
        let mut b = 0;

        for j in (1..=length).rev() {
            let mut c = i32::from(data[j - 1]);
            c = roll_left(c, 3);
            c ^= 0x13;
            a = c;
            c ^= b;
            c -= j as i32;
            c &= 0xff;
            c = roll_right(c, 4);
            b = a;
            data[j - 1] = c as u8;
        }

        b = 0;

        for j in (1..=length).rev() {
            let mut c = i32::from(data[length - j]);
            c -= 0x48;
            c &= 0xff;
            c ^= 0xff;
            c = roll_left(c, j as i32);
            a = c;
            c ^= b;
            c -= j as i32;
            c &= 0xff;
            c = roll_right(c, 3);
            b = a;
            data[length - j] = c as u8;
        }
    }

    data
}

fn roll_left(value: i32, shift: i32) -> i32 {
    let overflow = ((value >> 0) << shift % 8) >> 0;
    ((overflow & 0xff) | (overflow >> 8)) & 0xff
}

fn roll_right(value: i32, shift: i32) -> i32 {
    let overflow = ((value >> 0) << 8) >> shift % 8;
    ((overflow & 0xff) | (overflow >> 8)) & 0xff
}
