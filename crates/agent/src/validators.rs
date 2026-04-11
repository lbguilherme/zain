/// Valida CPF usando dígitos verificadores (Mod-11).
/// Aceita entrada com ou sem formatação (pontos, traços).
pub fn validar_cpf(cpf: &str) -> bool {
    let digits: Vec<u32> = cpf.chars().filter_map(|c| c.to_digit(10)).collect();

    if digits.len() != 11 {
        return false;
    }

    // Rejeita sequências repetidas (000.000.000-00, 111.111.111-11, etc.)
    if digits.windows(2).all(|w| w[0] == w[1]) {
        return false;
    }

    // Primeiro dígito verificador
    let sum: u32 = digits[..9]
        .iter()
        .enumerate()
        .map(|(i, &d)| d * (10 - i as u32))
        .sum();
    let expected1 = match sum % 11 {
        r if r < 2 => 0,
        r => 11 - r,
    };
    if digits[9] != expected1 {
        return false;
    }

    // Segundo dígito verificador
    let sum: u32 = digits[..10]
        .iter()
        .enumerate()
        .map(|(i, &d)| d * (11 - i as u32))
        .sum();
    let expected2 = match sum % 11 {
        r if r < 2 => 0,
        r => 11 - r,
    };
    digits[10] == expected2
}

/// Valida CNPJ usando dígitos verificadores (Mod-11).
/// Aceita entrada com ou sem formatação (pontos, barras, traços).
pub fn validar_cnpj(cnpj: &str) -> bool {
    let digits: Vec<u32> = cnpj.chars().filter_map(|c| c.to_digit(10)).collect();

    if digits.len() != 14 {
        return false;
    }

    // Rejeita sequências repetidas
    if digits.windows(2).all(|w| w[0] == w[1]) {
        return false;
    }

    // Primeiro dígito verificador
    let weights1: &[u32] = &[5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
    let sum: u32 = digits[..12]
        .iter()
        .zip(weights1)
        .map(|(&d, &w)| d * w)
        .sum();
    let expected1 = match sum % 11 {
        r if r < 2 => 0,
        r => 11 - r,
    };
    if digits[12] != expected1 {
        return false;
    }

    // Segundo dígito verificador
    let weights2: &[u32] = &[6, 5, 4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
    let sum: u32 = digits[..13]
        .iter()
        .zip(weights2)
        .map(|(&d, &w)| d * w)
        .sum();
    let expected2 = match sum % 11 {
        r if r < 2 => 0,
        r => 11 - r,
    };
    digits[13] == expected2
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpf_validos() {
        assert!(validar_cpf("52998224725"));
        assert!(validar_cpf("529.982.247-25")); // com formatação
    }

    #[test]
    fn cpf_invalidos() {
        assert!(!validar_cpf("00000000000"));
        assert!(!validar_cpf("11111111111"));
        assert!(!validar_cpf("12345678900"));
        assert!(!validar_cpf("123"));
        assert!(!validar_cpf(""));
    }

    #[test]
    fn cnpj_validos() {
        assert!(validar_cnpj("11222333000181"));
        assert!(validar_cnpj("11.222.333/0001-81")); // com formatação
    }

    #[test]
    fn cnpj_invalidos() {
        assert!(!validar_cnpj("00000000000000"));
        assert!(!validar_cnpj("11111111111111"));
        assert!(!validar_cnpj("12345678000100"));
        assert!(!validar_cnpj("123"));
        assert!(!validar_cnpj(""));
    }
}
