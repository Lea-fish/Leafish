/// A list of all supported versions
#[derive(PartialOrd, PartialEq, Debug, Copy, Clone)]
pub enum Version {
    Other,
    Old,
    V1_7,
    V1_8,
    V1_9,
    V1_10,
    V1_11,
    V1_12,
    V1_13,
    V1_13_2,
    V1_14,
    V1_15,
    V1_16,
    V1_16_2,
    V1_17,
    V1_18,
    V1_19,
    New,
}

impl Version {
    pub fn from_id(protocol_version: u32) -> Version {
        match protocol_version {
            0..=4 => Version::Old,
            5 => Version::V1_7,
            47 => Version::V1_8,
            107..=110 => Version::V1_9,
            210 => Version::V1_10,
            315..=316 => Version::V1_11,
            335..=340 => Version::V1_12,
            393..=401 => Version::V1_13,
            404..=404 => Version::V1_13_2,
            477..=498 => Version::V1_14,
            573..=578 => Version::V1_15,
            735..=736 => Version::V1_16,
            737..=754 => Version::V1_16_2,
            755..=756 => Version::V1_17,
            757..=758 => Version::V1_18,
            759..=760 => Version::V1_19,
            761..=u32::MAX => Version::New,
            _ => Version::Other,
        }
    }

    pub fn is_supported(&self) -> bool {
        match self {
            Version::Old => false,
            Version::New => false,
            Version::Other => false,
            Version::V1_17 => false,
            Version::V1_18 => false,
            Version::V1_19 => false,
            _ => true,
        }
    }
}
