/// Função de normalização para valores de coluna.
/// Recebe o valor trimado (nunca vazio) e retorna o valor para COPY ou erro.
pub type NormalizeFn = fn(&str) -> Result<String, &'static str>;

pub struct Column {
    pub name: &'static str,
    pub sql_type: &'static str,
    /// `None` = texto puro (escape_copy), `Some(f)` = normalização custom.
    pub normalize: Option<NormalizeFn>,
}

fn normalize_decimal(val: &str) -> Result<String, &'static str> {
    Ok(val.replace(',', "."))
}

impl Column {
    pub const fn text(name: &'static str, sql_type: &'static str) -> Self {
        Self { name, sql_type, normalize: None }
    }
    pub const fn int(name: &'static str, sql_type: &'static str) -> Self {
        Self { name, sql_type, normalize: None }
    }
    pub const fn decimal(name: &'static str, sql_type: &'static str) -> Self {
        Self { name, sql_type, normalize: Some(normalize_decimal) }
    }
    pub const fn date(name: &'static str, normalize: NormalizeFn) -> Self {
        Self { name, sql_type: "DATE", normalize: Some(normalize) }
    }
    pub const fn bool(name: &'static str, normalize: NormalizeFn) -> Self {
        Self { name, sql_type: "BOOLEAN", normalize: Some(normalize) }
    }
    pub const fn custom(name: &'static str, sql_type: &'static str, normalize: NormalizeFn) -> Self {
        Self { name, sql_type, normalize: Some(normalize) }
    }
}

pub struct Table {
    pub name: &'static str,
    pub file_prefix: &'static str,
    pub file_count: usize,
    pub columns: &'static [Column],
    pub extra_ddl: &'static [&'static str],
    /// CSVs dentro do zip têm linha de header
    pub has_headers: bool,
}

impl Table {
    pub fn zip_filenames(&self) -> Vec<String> {
        if self.file_count == 1 {
            vec![format!("{}.zip", self.file_prefix)]
        } else {
            (0..self.file_count)
                .map(|i| format!("{}{}.zip", self.file_prefix, i))
                .collect()
        }
    }

    pub fn create_table_sql(&self, schema: &str) -> String {
        let cols = self
            .columns
            .iter()
            .map(|c| {
                let sql_type = c.sql_type.replace("{schema}", schema);
                format!("  \"{}\" {}", c.name, sql_type)
            })
            .collect::<Vec<_>>()
            .join(",\n");
        format!("CREATE TABLE \"{schema}\".\"{}\" (\n{cols}\n)", self.name)
    }

    pub fn copy_in_sql(&self, schema: &str) -> String {
        let cols = self
            .columns
            .iter()
            .map(|c| format!("\"{}\"", c.name))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "COPY \"{schema}\".\"{}\" ({cols}) FROM STDIN WITH (FORMAT text, NULL '\\N')",
            self.name
        )
    }
}
