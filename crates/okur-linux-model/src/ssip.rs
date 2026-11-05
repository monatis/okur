//! speech-dispatcher's protocol (SSIP): the scales Okur's voice settings are given in.

/// SSIP's rate or pitch for Okur's: SSIP's run from -100 to 100 around the voice's own (0),
/// Okur's, as SAPI's, from -10 to 10. The ends and the middle meet; between them each Okur step
/// is ten of SSIP's. How fast a step sounds is each output module's own (espeak-ng's is not
/// SAPI's), to be measured against a SAPI voice before it is tuned.
pub fn ssip_scale(okur: i32) -> i32 {
    okur.clamp(-10, 10) * 10
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn okurs_scale_meets_ssips_at_the_ends_and_the_middle() {
        assert_eq!(ssip_scale(-10), -100);
        assert_eq!(ssip_scale(0), 0);
        assert_eq!(ssip_scale(10), 100);
        assert_eq!(ssip_scale(3), 30);
    }

    #[test]
    fn a_value_past_okurs_scale_is_held_to_its_end() {
        assert_eq!(ssip_scale(25), 100);
        assert_eq!(ssip_scale(-11), -100);
    }
}
