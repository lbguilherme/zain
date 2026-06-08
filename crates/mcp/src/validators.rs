/// Valida CPF usando dígitos verificadores (Mod-11).
/// Aceita entrada com ou sem formatação (pontos, traços).
pub fn validar_cpf(cpf: &str) -> bool {
    let digits: Vec<u32> = cpf.chars().filter_map(|c| c.to_digit(10)).collect();

    if digits.len() != 11 {
        return false;
    }

    if digits.windows(2).all(|w| w[0] == w[1]) {
        return false;
    }

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
