// Copyright (c) 2026 Meng Zhuo
// SPDX-License-Identifier: BSD-3-Clause

// Translated from musl: pow.c, powf.c, pow_data.c, powf_data.c
// Uses non-FMA paths throughout (no __FP_FAST_FMA).
// issignaling_inline/issignalingf_inline defined as 0 (always false) per musl libm.h.

// ============================================================
// pow (double precision) data
// ============================================================

const POW_LOG_TABLE_BITS: u32 = 7;
const POW_N: u64 = 1 << POW_LOG_TABLE_BITS;
const POW_OFF: u64 = 0x3FE6955500000000;

const POW_LN2HI: f64 = asdouble(0x3FE62E42FEFA3800);
const POW_LN2LO: f64 = asdouble(0x3D2EF35793C76730);

// Polynomial coefficients for log1p(r) - r - A[0]*r^2.
// First coefficient A[0] = -0.5 is implicit.
const POW_POLY: [f64; 7] = [
    asdouble(0xBFE0000000000000), // A[0] = -0.5
    asdouble(0xBFE5555555555560), // A[1]
    asdouble(0x3FE0000000000006), // A[2]
    asdouble(0x3FE999999959554E), // A[3]
    asdouble(0xBFE555555529A47A), // A[4]
    asdouble(0xBFF2495B9B4845E9), // A[5]
    asdouble(0x3FF0002B8B263FC3), // A[6]
];

// Table entries: (invc, logc, logctail)
const POW_LOG_TAB: [(f64, f64, f64); 128] = [
    (asdouble(0x3FF6A00000000000), asdouble(0xBFD62C82F2B9C800), asdouble(0x3CFAB42428375680)),
    (asdouble(0x3FF6800000000000), asdouble(0xBFD5D1BDBF580800), asdouble(0xBD1CA508D8E0F720)),
    (asdouble(0x3FF6600000000000), asdouble(0xBFD5767717455800), asdouble(0xBD2362A4D5B6506D)),
    (asdouble(0x3FF6400000000000), asdouble(0xBFD51AAD872DF800), asdouble(0xBCE684E49EB067D5)),
    (asdouble(0x3FF6200000000000), asdouble(0xBFD4BE5F95777800), asdouble(0xBD041B6993293EE0)),
    (asdouble(0x3FF6000000000000), asdouble(0xBFD4618BC21C6000), asdouble(0x3D13D82F484C84CC)),
    (asdouble(0x3FF5E00000000000), asdouble(0xBFD404308686A800), asdouble(0x3CDC42F3ED820B3A)),
    (asdouble(0x3FF5C00000000000), asdouble(0xBFD3A64C55694800), asdouble(0x3D20B1C686519460)),
    (asdouble(0x3FF5A00000000000), asdouble(0xBFD347DD9A988000), asdouble(0x3D25594DD4C58092)),
    (asdouble(0x3FF5800000000000), asdouble(0xBFD2E8E2BAE12000), asdouble(0x3D267B1E99B72BD8)),
    (asdouble(0x3FF5600000000000), asdouble(0xBFD2895A13DE8800), asdouble(0x3D15CA14B6CFB03F)),
    (asdouble(0x3FF5600000000000), asdouble(0xBFD2895A13DE8800), asdouble(0x3D15CA14B6CFB03F)),
    (asdouble(0x3FF5400000000000), asdouble(0xBFD22941FBCF7800), asdouble(0xBD165A242853DA76)),
    (asdouble(0x3FF5200000000000), asdouble(0xBFD1C898C1699800), asdouble(0xBD1FAFBC68E75404)),
    (asdouble(0x3FF5000000000000), asdouble(0xBFD1675CABABA800), asdouble(0x3D1F1FC63382A8F0)),
    (asdouble(0x3FF4E00000000000), asdouble(0xBFD1058BF9AE4800), asdouble(0xBD26A8C4FD055A66)),
    (asdouble(0x3FF4C00000000000), asdouble(0xBFD0A324E2739000), asdouble(0xBD0C6BEE7EF4030E)),
    (asdouble(0x3FF4A00000000000), asdouble(0xBFD0402594B4D000), asdouble(0xBCF036B89EF42D7F)),
    (asdouble(0x3FF4A00000000000), asdouble(0xBFD0402594B4D000), asdouble(0xBCF036B89EF42D7F)),
    (asdouble(0x3FF4800000000000), asdouble(0xBFCFB9186D5E4000), asdouble(0x3D0D572AAB993C87)),
    (asdouble(0x3FF4600000000000), asdouble(0xBFCEF0ADCBDC6000), asdouble(0x3D2B26B79C86AF24)),
    (asdouble(0x3FF4400000000000), asdouble(0xBFCE27076E2AF000), asdouble(0xBD172F4F543FFF10)),
    (asdouble(0x3FF4200000000000), asdouble(0xBFCD5C216B4FC000), asdouble(0x3D21BA91BBCA681B)),
    (asdouble(0x3FF4000000000000), asdouble(0xBFCC8FF7C79AA000), asdouble(0x3D27794F689F8434)),
    (asdouble(0x3FF4000000000000), asdouble(0xBFCC8FF7C79AA000), asdouble(0x3D27794F689F8434)),
    (asdouble(0x3FF3E00000000000), asdouble(0xBFCBC286742D9000), asdouble(0x3D194EB0318BB78F)),
    (asdouble(0x3FF3C00000000000), asdouble(0xBFCAF3C94E80C000), asdouble(0x3CBA4E633FCD9066)),
    (asdouble(0x3FF3A00000000000), asdouble(0xBFCA23BC1FE2B000), asdouble(0xBD258C64DC46C1EA)),
    (asdouble(0x3FF3A00000000000), asdouble(0xBFCA23BC1FE2B000), asdouble(0xBD258C64DC46C1EA)),
    (asdouble(0x3FF3800000000000), asdouble(0xBFC9525A9CF45000), asdouble(0xBD2AD1D904C1D4E3)),
    (asdouble(0x3FF3600000000000), asdouble(0xBFC87FA06520D000), asdouble(0x3D2BBDBF7FDBFA09)),
    (asdouble(0x3FF3400000000000), asdouble(0xBFC7AB890210E000), asdouble(0x3D2BDB9072534A58)),
    (asdouble(0x3FF3400000000000), asdouble(0xBFC7AB890210E000), asdouble(0x3D2BDB9072534A58)),
    (asdouble(0x3FF3200000000000), asdouble(0xBFC6D60FE719D000), asdouble(0xBD10E46AA3B2E266)),
    (asdouble(0x3FF3000000000000), asdouble(0xBFC5FF3070A79000), asdouble(0xBD1E9E439F105039)),
    (asdouble(0x3FF3000000000000), asdouble(0xBFC5FF3070A79000), asdouble(0xBD1E9E439F105039)),
    (asdouble(0x3FF2E00000000000), asdouble(0xBFC526E5E3A1B000), asdouble(0xBD20DE8B90075B8F)),
    (asdouble(0x3FF2C00000000000), asdouble(0xBFC44D2B6CCB8000), asdouble(0x3D170CC16135783C)),
    (asdouble(0x3FF2C00000000000), asdouble(0xBFC44D2B6CCB8000), asdouble(0x3D170CC16135783C)),
    (asdouble(0x3FF2A00000000000), asdouble(0xBFC371FC201E9000), asdouble(0x3CF178864D27543A)),
    (asdouble(0x3FF2800000000000), asdouble(0xBFC29552F81FF000), asdouble(0xBD248D301771C408)),
    (asdouble(0x3FF2600000000000), asdouble(0xBFC1B72AD52F6000), asdouble(0xBD2E80A41811A396)),
    (asdouble(0x3FF2600000000000), asdouble(0xBFC1B72AD52F6000), asdouble(0xBD2E80A41811A396)),
    (asdouble(0x3FF2400000000000), asdouble(0xBFC0D77E7CD09000), asdouble(0x3D0A699688E85BF4)),
    (asdouble(0x3FF2400000000000), asdouble(0xBFC0D77E7CD09000), asdouble(0x3D0A699688E85BF4)),
    (asdouble(0x3FF2200000000000), asdouble(0xBFBFEC9131DBE000), asdouble(0xBD2575545CA333F2)),
    (asdouble(0x3FF2000000000000), asdouble(0xBFBE27076E2B0000), asdouble(0x3D2A342C2AF0003C)),
    (asdouble(0x3FF2000000000000), asdouble(0xBFBE27076E2B0000), asdouble(0x3D2A342C2AF0003C)),
    (asdouble(0x3FF1E00000000000), asdouble(0xBFBC5E548F5BC000), asdouble(0xBD1D0C57585FBE06)),
    (asdouble(0x3FF1C00000000000), asdouble(0xBFBA926D3A4AE000), asdouble(0x3D253935E85BAAC8)),
    (asdouble(0x3FF1C00000000000), asdouble(0xBFBA926D3A4AE000), asdouble(0x3D253935E85BAAC8)),
    (asdouble(0x3FF1A00000000000), asdouble(0xBFB8C345D631A000), asdouble(0x3D137C294D2F5668)),
    (asdouble(0x3FF1A00000000000), asdouble(0xBFB8C345D631A000), asdouble(0x3D137C294D2F5668)),
    (asdouble(0x3FF1800000000000), asdouble(0xBFB6F0D28AE56000), asdouble(0xBD269737C93373DA)),
    (asdouble(0x3FF1600000000000), asdouble(0xBFB51B073F062000), asdouble(0x3D1F025B61C65E57)),
    (asdouble(0x3FF1600000000000), asdouble(0xBFB51B073F062000), asdouble(0x3D1F025B61C65E57)),
    (asdouble(0x3FF1400000000000), asdouble(0xBFB341D7961BE000), asdouble(0x3D2C5EDACCF913DF)),
    (asdouble(0x3FF1400000000000), asdouble(0xBFB341D7961BE000), asdouble(0x3D2C5EDACCF913DF)),
    (asdouble(0x3FF1200000000000), asdouble(0xBFB16536EEA38000), asdouble(0x3D147C5E768FA309)),
    (asdouble(0x3FF1000000000000), asdouble(0xBFAF0A30C0118000), asdouble(0x3D2D599E83368E91)),
    (asdouble(0x3FF1000000000000), asdouble(0xBFAF0A30C0118000), asdouble(0x3D2D599E83368E91)),
    (asdouble(0x3FF0E00000000000), asdouble(0xBFAB42DD71198000), asdouble(0x3D1C827AE5D6704C)),
    (asdouble(0x3FF0E00000000000), asdouble(0xBFAB42DD71198000), asdouble(0x3D1C827AE5D6704C)),
    (asdouble(0x3FF0C00000000000), asdouble(0xBFA77458F632C000), asdouble(0xBD2CFC4634F2A1EE)),
    (asdouble(0x3FF0C00000000000), asdouble(0xBFA77458F632C000), asdouble(0xBD2CFC4634F2A1EE)),
    (asdouble(0x3FF0A00000000000), asdouble(0xBFA39E87B9FEC000), asdouble(0x3CF502B7F526FEAA)),
    (asdouble(0x3FF0A00000000000), asdouble(0xBFA39E87B9FEC000), asdouble(0x3CF502B7F526FEAA)),
    (asdouble(0x3FF0800000000000), asdouble(0xBF9F829B0E780000), asdouble(0xBD2980267C7E09E4)),
    (asdouble(0x3FF0800000000000), asdouble(0xBF9F829B0E780000), asdouble(0xBD2980267C7E09E4)),
    (asdouble(0x3FF0600000000000), asdouble(0xBF97B91B07D58000), asdouble(0xBD288D5493FAA639)),
    (asdouble(0x3FF0400000000000), asdouble(0xBF8FC0A8B0FC0000), asdouble(0xBCDF1E7CF6D3A69C)),
    (asdouble(0x3FF0400000000000), asdouble(0xBF8FC0A8B0FC0000), asdouble(0xBCDF1E7CF6D3A69C)),
    (asdouble(0x3FF0200000000000), asdouble(0xBF7FE02A6B100000), asdouble(0xBD19E23F0DDA40E4)),
    (asdouble(0x3FF0200000000000), asdouble(0xBF7FE02A6B100000), asdouble(0xBD19E23F0DDA40E4)),
    (asdouble(0x3FF0000000000000), asdouble(0x0000000000000000), asdouble(0x0000000000000000)),
    (asdouble(0x3FF0000000000000), asdouble(0x0000000000000000), asdouble(0x0000000000000000)),
    (asdouble(0x3FEFC00000000000), asdouble(0x3F80101575890000), asdouble(0xBD10C76B999D2BE8)),
    (asdouble(0x3FEF800000000000), asdouble(0x3F90205658938000), asdouble(0xBD23DC5B06E2F7D2)),
    (asdouble(0x3FEF400000000000), asdouble(0x3F98492528C90000), asdouble(0xBD2AA0BA325A0C34)),
    (asdouble(0x3FEF000000000000), asdouble(0x3FA0415D89E74000), asdouble(0x3D0111C05CF1D753)),
    (asdouble(0x3FEEC00000000000), asdouble(0x3FA466AED42E0000), asdouble(0xBD2C167375BDFD28)),
    (asdouble(0x3FEE800000000000), asdouble(0x3FA894AA149FC000), asdouble(0xBD197995D05A267D)),
    (asdouble(0x3FEE400000000000), asdouble(0x3FACCB73CDDDC000), asdouble(0xBD1A68F247D82807)),
    (asdouble(0x3FEE200000000000), asdouble(0x3FAEEA31C006C000), asdouble(0xBD0E113E4FC93B7B)),
    (asdouble(0x3FEDE00000000000), asdouble(0x3FB1973BD1466000), asdouble(0xBD25325D560D9E9B)),
    (asdouble(0x3FEDA00000000000), asdouble(0x3FB3BDF5A7D1E000), asdouble(0x3D2CC85EA5DB4ED7)),
    (asdouble(0x3FED600000000000), asdouble(0x3FB5E95A4D97A000), asdouble(0xBD2C69063C5D1D1E)),
    (asdouble(0x3FED400000000000), asdouble(0x3FB700D30AEAC000), asdouble(0x3CEC1E8DA99DED32)),
    (asdouble(0x3FED000000000000), asdouble(0x3FB9335E5D594000), asdouble(0x3D23115C3ABD47DA)),
    (asdouble(0x3FECC00000000000), asdouble(0x3FBB6AC88DAD6000), asdouble(0xBD1390802BF768E5)),
    (asdouble(0x3FECA00000000000), asdouble(0x3FBC885801BC4000), asdouble(0x3D2646D1C65AACD3)),
    (asdouble(0x3FEC600000000000), asdouble(0x3FBEC739830A2000), asdouble(0xBD2DC068AFE645E0)),
    (asdouble(0x3FEC400000000000), asdouble(0x3FBFE89139DBE000), asdouble(0xBD2534D64FA10AFD)),
    (asdouble(0x3FEC000000000000), asdouble(0x3FC1178E8227E000), asdouble(0x3D21EF78CE2D07F2)),
    (asdouble(0x3FEBE00000000000), asdouble(0x3FC1AA2B7E23F000), asdouble(0x3D2CA78E44389934)),
    (asdouble(0x3FEBA00000000000), asdouble(0x3FC2D1610C868000), asdouble(0x3D039D6CCB81B4A1)),
    (asdouble(0x3FEB800000000000), asdouble(0x3FC365FCB0159000), asdouble(0x3CC62FA8234B7289)),
    (asdouble(0x3FEB400000000000), asdouble(0x3FC4913D8333B000), asdouble(0x3D25837954FDB678)),
    (asdouble(0x3FEB200000000000), asdouble(0x3FC527E5E4A1B000), asdouble(0x3D2633E8E5697DC7)),
    (asdouble(0x3FEAE00000000000), asdouble(0x3FC6574EBE8C1000), asdouble(0x3D19CF8B2C3C2E78)),
    (asdouble(0x3FEAC00000000000), asdouble(0x3FC6F0128B757000), asdouble(0xBD25118DE59C21E1)),
    (asdouble(0x3FEAA00000000000), asdouble(0x3FC7898D85445000), asdouble(0xBD1C661070914305)),
    (asdouble(0x3FEA600000000000), asdouble(0x3FC8BEAFEB390000), asdouble(0xBD073D54AAE92CD1)),
    (asdouble(0x3FEA400000000000), asdouble(0x3FC95A5ADCF70000), asdouble(0x3D07F22858A0FF6F)),
    (asdouble(0x3FEA000000000000), asdouble(0x3FCA93ED3C8AE000), asdouble(0xBD28724350562169)),
    (asdouble(0x3FE9E00000000000), asdouble(0x3FCB31D8575BD000), asdouble(0xBD0C358D4EACE1AA)),
    (asdouble(0x3FE9C00000000000), asdouble(0x3FCBD087383BE000), asdouble(0xBD2D4BC4595412B6)),
    (asdouble(0x3FE9A00000000000), asdouble(0x3FCC6FFBC6F01000), asdouble(0xBCF1EC72C5962BD2)),
    (asdouble(0x3FE9600000000000), asdouble(0x3FCDB13DB0D49000), asdouble(0xBD2AFF2AF715B035)),
    (asdouble(0x3FE9400000000000), asdouble(0x3FCE530EFFE71000), asdouble(0x3CC212276041F430)),
    (asdouble(0x3FE9200000000000), asdouble(0x3FCEF5ADE4DD0000), asdouble(0xBCCA211565BB8E11)),
    (asdouble(0x3FE9000000000000), asdouble(0x3FCF991C6CB3B000), asdouble(0x3D1BCBECCA0CDF30)),
    (asdouble(0x3FE8C00000000000), asdouble(0x3FD07138604D5800), asdouble(0x3CF89CDB16ED4E91)),
    (asdouble(0x3FE8A00000000000), asdouble(0x3FD0C42D67616000), asdouble(0x3D27188B163CEAE9)),
    (asdouble(0x3FE8800000000000), asdouble(0x3FD1178E8227E800), asdouble(0xBD2C210E63A5F01C)),
    (asdouble(0x3FE8600000000000), asdouble(0x3FD16B5CCBACF800), asdouble(0x3D2B9ACDF7A51681)),
    (asdouble(0x3FE8400000000000), asdouble(0x3FD1BF99635A6800), asdouble(0x3D2CA6ED5147BDB7)),
    (asdouble(0x3FE8200000000000), asdouble(0x3FD214456D0EB800), asdouble(0x3D0A87DEBA46BAEA)),
    (asdouble(0x3FE7E00000000000), asdouble(0x3FD2BEF07CDC9000), asdouble(0x3D2A9CFA4A5004F4)),
    (asdouble(0x3FE7C00000000000), asdouble(0x3FD314F1E1D36000), asdouble(0xBD28E27AD3213CB8)),
    (asdouble(0x3FE7A00000000000), asdouble(0x3FD36B6776BE1000), asdouble(0x3D116ECDB0F177C8)),
    (asdouble(0x3FE7800000000000), asdouble(0x3FD3C25277333000), asdouble(0x3D183B54B606BD5C)),
    (asdouble(0x3FE7600000000000), asdouble(0x3FD419B423D5E800), asdouble(0x3D08E436EC90E09D)),
    (asdouble(0x3FE7400000000000), asdouble(0x3FD4718DC271C800), asdouble(0xBD2F27CE0967D675)),
    (asdouble(0x3FE7200000000000), asdouble(0x3FD4C9E09E173000), asdouble(0xBD2E20891B0AD8A4)),
    (asdouble(0x3FE7000000000000), asdouble(0x3FD522AE0738A000), asdouble(0x3D2EBE708164C759)),
    (asdouble(0x3FE6E00000000000), asdouble(0x3FD57BF753C8D000), asdouble(0x3D1FADEDEE5D40EF)),
    (asdouble(0x3FE6C00000000000), asdouble(0x3FD5D5BDDF596000), asdouble(0xBD0A0B2A08A465DC)),
];

// ============================================================
// exp_inline shares the exp/exp2/log2 tables from the other math files
// ============================================================

const POW_EXP_N: u64 = 1 << EXP_TABLE_BITS;
const POW_SIGN_BIAS: u64 = 0x800 << EXP_TABLE_BITS; // 0x40000

const POW_INVLN2N: f64 = asdouble(0x40671547652B82FE);
const POW_NEGLN2HI_N: f64 = asdouble(0xBF762E42FEFA0000);
const POW_NEGLN2LO_N: f64 = asdouble(0xBD0CF79ABC9E3B3A);
const POW_SHIFT: f64 = asdouble(0x4338000000000000);
const POW_1P1009: f64 = asdouble(0x7F00000000000000);
const POW_1P_NEG1022: f64 = asdouble(0x0010000000000000);

const POW_EXP_C2: f64 = asdouble(0x3FDFFFFFFFFFFDBD);
const POW_EXP_C3: f64 = asdouble(0x3FC555555555543C);
const POW_EXP_C4: f64 = asdouble(0x3FA55555CF172B91);
const POW_EXP_C5: f64 = asdouble(0x3F81111167A4D017);

// ============================================================
// pow helpers
// ============================================================

/// Top 12 bits of a double (sign and exponent bits).
#[inline]
fn pow_top12(x: f64) -> u32 {
    (asuint64(x) >> 52) as u32
}

/// Compute y+tail = log(x) where the rounded result is y and tail has about
/// additional 15 bits precision. IX is the bit representation of x, but
/// normalized in the subnormal range using the sign bit for the exponent.
/// Uses non-FMA path.
#[inline]
fn pow_log_inline(ix: u64, tail: &mut f64) -> f64 {
    let tmp = ix.wrapping_sub(POW_OFF);
    let i = ((tmp >> (52 - POW_LOG_TABLE_BITS)) % POW_N) as usize;
    let k = (tmp as i64 >> 52) as i32;
    let iz = ix - (tmp & (0xFFFu64 << 52));
    let z = asdouble(iz);
    let kd = k as f64;

    let invc = POW_LOG_TAB[i].0;
    let logc = POW_LOG_TAB[i].1;
    let logctail = POW_LOG_TAB[i].2;

    // Non-FMA path: split z for accurate r = z/c - 1
    let zhi = asdouble((iz.wrapping_add(1u64 << 31)) & (!0u64 << 32));
    let zlo = z - zhi;
    let rhi = zhi * invc - 1.0;
    let rlo = zlo * invc;
    let r = rhi + rlo;

    // k*Ln2 + log(c) + r
    let t1 = kd * POW_LN2HI + logc;
    let t2 = t1 + r;
    let lo1 = kd * POW_LN2LO + logctail;
    let lo2 = t1 - t2 + r;

    // Polynomial: p = log1p(r) - r - A[0]*r^2
    let ar = POW_POLY[0] * r;
    let ar2 = r * ar;
    let ar3 = r * ar2;

    // Non-FMA path for hi/lo3/lo4
    let arhi = POW_POLY[0] * rhi;
    let arhi2 = rhi * arhi;
    let hi = t2 + arhi2;
    let lo3 = rlo * (ar + arhi);
    let lo4 = t2 - hi + arhi2;

    let p = ar3 * (POW_POLY[1]
        + r * POW_POLY[2]
        + ar2 * (POW_POLY[3] + r * POW_POLY[4] + ar2 * (POW_POLY[5] + r * POW_POLY[6])));
    let lo = lo1 + lo2 + lo3 + lo4 + p;
    let y = hi + lo;
    *tail = hi - y + lo;
    y
}

/// Handle cases that may overflow or underflow when computing the result
/// that is scale*(1+tmp) without intermediate rounding.
#[inline]
fn pow_exp_specialcase(tmp: f64, sbits: u64, ki: u64) -> f64 {
    if (ki & 0x80000000) == 0 {
        // k > 0, the exponent of scale might have overflowed by <= 460.
        let sbits = sbits.wrapping_sub(1009u64 << 52);
        let scale = asdouble(sbits);
        let y = POW_1P1009 * (scale + scale * tmp);
        return eval_as_double(y);
    }
    // k < 0, need special care in the subnormal range.
    let sbits = sbits.wrapping_add(1022u64 << 52);
    let scale = asdouble(sbits);
    let mut y = scale + scale * tmp;
    if y.abs() < 1.0 {
        // Round y to the right precision before scaling into subnormal range.
        let mut lo = scale - y + scale * tmp;
        let mut one: f64 = 1.0;
        if y < 0.0 {
            one = -1.0;
        }
        let hi = one + y;
        lo = one - hi + y + lo;
        y = eval_as_double(hi + lo) - one;
        // Fix the sign of 0.
        if y == 0.0 {
            y = asdouble(sbits & 0x8000000000000000);
            // The rounded result is 0, so the final multiplication would be
            // exact and would not signal underflow. Signal it explicitly.
            fp_force_eval(fp_barrier(POW_1P_NEG1022) * POW_1P_NEG1022);
        } else {
            // If the scaled result is inexact-subnormal or underflows to 0,
            // the multiplication raises UNDERFLOW; if it is exact, it does not.
            fp_force_eval(POW_1P_NEG1022 * y);
        }
    }
    y = POW_1P_NEG1022 * y;
    eval_as_double(y)
}

/// Finish `pow` from the reduced log product: split y, hi and lo into
/// high/low halves so the final exp kernel sees ~2^-27 tails.
#[inline]
fn pow_finish(iy: u64, y: f64, hi: f64, lo: f64, sign_bias: u64) -> f64 {
    let yhi = asdouble(iy & (!0u64 << 27));
    let ylo = y - yhi;
    let lhi = asdouble(asuint64(hi) & (!0u64 << 27));
    let llo = hi - lhi + lo;
    let ehi = yhi * lhi;
    let elo = ylo * lhi + y * llo;
    pow_exp_inline(ehi, elo, sign_bias)
}

/// Computes sign*exp(x+xtail) where |xtail| < 2^-8/N and |xtail| <= |x|.
/// The sign_bias argument is POW_SIGN_BIAS or 0 and sets the sign to -1 or 1.
#[inline]
fn pow_exp_inline(x: f64, xtail: f64, sign_bias: u64) -> f64 {
    exp_core(x, xtail, sign_bias, true)

}

/// Returns 0 if not int, 1 if odd int, 2 if even int.
/// The argument is the bit representation of a non-zero finite floating-point value.
#[inline]
fn pow_checkint(iy: u64) -> i32 {
    let e = (iy >> 52) & 0x7ff;
    if e < 0x3ff {
        return 0;
    }
    if e > 0x3ff + 52 {
        return 2;
    }
    if (iy & ((1u64 << (0x3ff + 52 - e)) - 1)) != 0 {
        return 0;
    }
    if (iy & (1u64 << (0x3ff + 52 - e))) != 0 {
        return 1;
    }
    2
}

/// Returns true if input is the bit representation of 0, infinity or nan.
#[inline]
fn pow_zeroinfnan(i: u64) -> bool {
    2u64.wrapping_mul(i).wrapping_sub(1) >= 2u64.wrapping_mul(asuint64(f64::INFINITY)).wrapping_sub(1)
}

// ============================================================
// pow (double precision)
// ============================================================


// Kept out of line: every path in here returns, and it is only entered for
// subnormal/non-finite inputs or extreme exponents.
#[inline(never)]
fn pow_special(x: f64, y: f64) -> f64 {
    let ix = asuint64(x);
    let iy = asuint64(y);
    let mut topx = pow_top12(x);
    let topy = pow_top12(y);
    let mut sign_bias: u64 = 0;

    // Note: if |y| > 1075*ln2*2^53 then pow(x,y) = inf/0
    // and if |y| < 2^-54/1075 then pow(x,y) = +-1.
    // Special cases: (x < 0x1p-126 or inf or nan) or
    // (|y| < 0x1p-65 or |y| >= 0x1p63 or nan).
    if predict_false(pow_zeroinfnan(iy)) {
        if 2u64.wrapping_mul(iy) == 0 {
            // issignaling_inline(x) is always false
            return 1.0;
        }
        if ix == asuint64(1.0) {
            // issignaling_inline(y) is always false
            return 1.0;
        }
        if 2u64.wrapping_mul(ix) > 2u64.wrapping_mul(asuint64(f64::INFINITY))
            || 2u64.wrapping_mul(iy) > 2u64.wrapping_mul(asuint64(f64::INFINITY))
        {
            return x + y;
        }
        if 2u64.wrapping_mul(ix) == 2u64.wrapping_mul(asuint64(1.0)) {
            return 1.0;
        }
        if (2u64.wrapping_mul(ix) < 2u64.wrapping_mul(asuint64(1.0)))
            == !(iy >> 63 != 0)
        {
            return 0.0; // |x|<1 && y==inf or |x|>1 && y==-inf.
        }
        return y * y;
    }
    if predict_false(pow_zeroinfnan(ix)) {
        let mut x2 = x * x;
        if ix >> 63 != 0 && pow_checkint(iy) == 1 {
            x2 = -x2;
        }
        // Without the barrier some versions of clang hoist the 1/x2 and
        // thus division by zero exception can be signaled spuriously.
        return if iy >> 63 != 0 {
            fp_barrier(1.0 / x2)
        } else {
            x2
        };
    }
    // Here x and y are non-zero finite.
    let mut ix = ix;
    if ix >> 63 != 0 {
        // Finite x < 0.
        let yint = pow_checkint(iy);
        if yint == 0 {
            return __math_invalid(x);
        }
        if yint == 1 {
            sign_bias = POW_SIGN_BIAS;
        }
        ix &= 0x7fffffffffffffff;
        topx &= 0x7ff;
    }
    if (topy & 0x7ff).wrapping_sub(0x3be) >= 0x43e - 0x3be {
        // Note: sign_bias == 0 here because y is not odd.
        if ix == asuint64(1.0) {
            return 1.0;
        }
        if (topy & 0x7ff) < 0x3be {
            // |y| < 2^-65, x^y ~= 1 + y*log(x).
            return if ix > asuint64(1.0) {
                1.0 + y
            } else {
                1.0 - y
            };
        }
        return if (ix > asuint64(1.0)) == (topy < 0x800) {
            __math_oflow(0)
        } else {
            __math_uflow(0)
        };
    }
    if topx == 0 {
        // Normalize subnormal x so exponent becomes negative.
        let mut ix = asuint64(x * asdouble(0x4330000000000000)); // 0x1p52
        ix &= 0x7fffffffffffffff;
        ix = ix.wrapping_sub(52u64 << 52);
        // Fall through to log_inline/exp_inline with normalized ix.
        let mut lo: f64 = 0.0;
        let hi = pow_log_inline(ix, &mut lo);
        return pow_finish(iy, y, hi, lo, sign_bias);
    }
    // Non-subnormal, non-special x.
    let mut lo: f64 = 0.0;
    let hi = pow_log_inline(ix, &mut lo);
    return pow_finish(iy, y, hi, lo, sign_bias);

}

#[no_mangle]
pub extern "C" fn pow(x: f64, y: f64) -> f64 {
    let sign_bias: u64 = 0;
    let ix = asuint64(x);
    let iy = asuint64(y);
    let topx = pow_top12(x);
    let topy = pow_top12(y);

    // Exact identity: x^1 is x for all finite/infinite/NaN x, with no exceptions.
    if iy == asuint64(1.0) {
        return x;
    }
    // Exact identity: x^-1 is 1/x, letting the division raise the correct flags.
    if iy == asuint64(-1.0) {
        return 1.0 / x;
    }

    if predict_false(
        topx.wrapping_sub(0x001) >= 0x7ff - 0x001
            || (topy & 0x7ff).wrapping_sub(0x3be) >= 0x43e - 0x3be,
    ) {
        return pow_special(x, y);
    }

    // Fast path: both x and y are normal, non-special.
    let mut lo: f64 = 0.0;
    let hi = pow_log_inline(ix, &mut lo);
    pow_finish(iy, y, hi, lo, sign_bias)
}

// ============================================================
// powf (single precision) data
// ============================================================

const POWF_N: u64 = 1 << LOG2F_TABLE_BITS;
const POWF_OFF: u32 = 0x3F330000;

const POWF_LOG2_POLY: [f64; 5] = [
    asdouble(0x3FD27616C9496E0B),
    asdouble(0xBFD71969A075C67A),
    asdouble(0x3FDEC70A6CA7BADD),
    asdouble(0xBFE7154748BEF6C8),
    asdouble(0x3FF71547652AB82B),
];

// ============================================================
// Duplicate exp2f data for powf's exp2_inline (self-contained)
// ============================================================

const POWF_N_EXP2F: u64 = 1 << EXP2F_TABLE_BITS;
const POWF_SIGN_BIAS: u64 = 1 << (EXP2F_TABLE_BITS + 11); // 0x10000

const POWF_EXP2F_SHIFT_SCALED: f64 = asdouble(0x42E8000000000000);
const POWF_EXP2F_POLY: [f64; 3] = [
    asdouble(0x3FAC6AF84B912394),
    asdouble(0x3FCEBFCE50FAC4F3),
    asdouble(0x3FE62E42FF0C52D6),
];

// ============================================================
// powf helpers
// ============================================================

/// log2(x) for powf. IX is the bit representation of x, normalized for subnormals.
#[inline]
fn powf_log2_inline(ix: u32) -> f64 {
    let tmp = ix.wrapping_sub(POWF_OFF);
    let i = ((tmp >> (23 - LOG2F_TABLE_BITS)) % (POWF_N as u32)) as usize;
    let top = tmp & 0xff800000;
    let iz = ix - top;
    let k = (tmp as i32 >> 23) as i32;
    let invc = LOG2F_TAB[i].0;
    let logc = LOG2F_TAB[i].1;
    let z = asfloat(iz) as f64;

    // log2(x) = log1p(z/c-1)/ln2 + log2(c) + k
    let r = z * invc - 1.0;
    let y0 = logc + k as f64;

    // Pipelined polynomial evaluation to approximate log1p(r)/ln2.
    let r2 = r * r;
    let y = POWF_LOG2_POLY[0] * r + POWF_LOG2_POLY[1];
    let p = POWF_LOG2_POLY[2] * r + POWF_LOG2_POLY[3];
    let r4 = r2 * r2;
    let q = POWF_LOG2_POLY[4] * r + y0;
    let q = p * r2 + q;
    let y = y * r4 + q;
    y
}

/// exp2(xd) for powf. xd is double precision; sign_bias sets the sign of the result.
#[inline]
fn powf_exp2_inline(xd: f64, sign_bias: u64) -> f32 {
    // x = k/N + r with r in [-1/(2N), 1/(2N)]
    let kd = eval_as_double(xd + POWF_EXP2F_SHIFT_SCALED);
    let ki = asuint64(kd);
    let kd = kd - POWF_EXP2F_SHIFT_SCALED; // k/N
    let r = xd - kd;

    // exp2(x) = 2^(k/N) * 2^r ~= s * (C0*r^3 + C1*r^2 + C2*r + 1)
    let t = EXP2F_TAB[(ki % POWF_N_EXP2F) as usize]
        .wrapping_add((ki.wrapping_add(sign_bias)) << (52 - EXP2F_TABLE_BITS));
    let s = asdouble(t);
    let z = POWF_EXP2F_POLY[0] * r + POWF_EXP2F_POLY[1];
    let r2 = r * r;
    let y = POWF_EXP2F_POLY[2] * r + 1.0;
    let y = z * r2 + y;
    let y = y * s;
    eval_as_float(y as f32)
}

/// Returns 0 if not int, 1 if odd int, 2 if even int.
#[inline]
fn powf_checkint(iy: u32) -> i32 {
    let e = (iy >> 23) & 0xff;
    if e < 0x7f {
        return 0;
    }
    if e > 0x7f + 23 {
        return 2;
    }
    if (iy & ((1u32 << (0x7f + 23 - e)) - 1)) != 0 {
        return 0;
    }
    if (iy & (1u32 << (0x7f + 23 - e))) != 0 {
        return 1;
    }
    2
}

/// Returns true if input is the bit representation of 0, infinity or nan.
#[inline]
fn powf_zeroinfnan(ix: u32) -> bool {
    2u32.wrapping_mul(ix).wrapping_sub(1) >= 2u32.wrapping_mul(0x7f800000u32).wrapping_sub(1)
}

// ============================================================
// powf (single precision)
// ============================================================

const POWF_OVFL_THRESHOLD: f64 = asdouble(0x405FFFFFFFD1D571);
const POWF_UVFL_THRESHOLD: f64 = asdouble(0xC062C00000000000);

#[no_mangle]
pub extern "C" fn powf(x: f32, y: f32) -> f32 {
    let mut sign_bias: u64 = 0;
    let mut ix = asuint(x);
    let iy = asuint(y);

    // Exact identity: x^1 is x for all finite/infinite/NaN x, with no exceptions.
    if iy == 0x3f800000 {
        return x;
    }
    // Exact identity: x^-1 is 1/x, letting the division raise the correct flags.
    if iy == 0xbf800000 {
        return 1.0 / x;
    }

    if predict_false(
        ix.wrapping_sub(0x00800000) >= 0x7f800000 - 0x00800000 || powf_zeroinfnan(iy),
    ) {
        // Either (x < 0x1p-126 or inf or nan) or (y is 0 or inf or nan).
        if predict_false(powf_zeroinfnan(iy)) {
            if 2u32.wrapping_mul(iy) == 0 {
                // issignalingf_inline(x) is always false
                return 1.0;
            }
            if ix == 0x3f800000 {
                // issignalingf_inline(y) is always false
                return 1.0;
            }
            if 2u32.wrapping_mul(ix) > 2u32.wrapping_mul(0x7f800000u32)
                || 2u32.wrapping_mul(iy) > 2u32.wrapping_mul(0x7f800000u32)
            {
                return x + y;
            }
            if 2u32.wrapping_mul(ix) == 2u32.wrapping_mul(0x3f800000u32) {
                return 1.0;
            }
            if (2u32.wrapping_mul(ix) < 2u32.wrapping_mul(0x3f800000u32))
                == !(iy & 0x80000000 != 0)
            {
                return 0.0; // |x|<1 && y==inf or |x|>1 && y==-inf.
            }
            return y * y;
        }
        if predict_false(powf_zeroinfnan(ix)) {
            let mut x2 = x * x;
            if ix & 0x80000000 != 0 && powf_checkint(iy) == 1 {
                x2 = -x2;
            }
            return if iy & 0x80000000 != 0 {
                fp_barrierf(1.0 / x2)
            } else {
                x2
            };
        }
        // x and y are non-zero finite.
        if ix & 0x80000000 != 0 {
            // Finite x < 0.
            let yint = powf_checkint(iy);
            if yint == 0 {
                return __math_invalidf(x);
            }
            if yint == 1 {
                sign_bias = POWF_SIGN_BIAS;
            }
            ix &= 0x7fffffff;
        }
        if ix < 0x00800000 {
            // Normalize subnormal x so exponent becomes negative.
            ix = asuint(x * asfloat(0x4B000000)); // 0x1p23f
            ix &= 0x7fffffff;
            ix = ix.wrapping_sub(23u32 << 23);
        }
    }

    let logx = powf_log2_inline(ix);
    let ylogx = y as f64 * logx; // cannot overflow, y is single prec.
    if predict_false(
        (asuint64(ylogx) >> 47 & 0xffff) >= (asuint64(126.0) >> 47),
    ) {
        // |y*log(x)| >= 126.
        if ylogx > POWF_OVFL_THRESHOLD {
            return __math_oflowf(sign_bias as u32);
        }
        if ylogx <= POWF_UVFL_THRESHOLD {
            return __math_uflowf(sign_bias as u32);
        }
    }
    powf_exp2_inline(ylogx, sign_bias)
}
