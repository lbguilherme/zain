use serde_json::{Value, json};

/// Definição de uma tool para enviar ao Ollama.
pub struct ToolDef {
    pub name: &'static str,
    pub description: &'static str,
    pub parameters: Value,
    /// Tools com efeitos externos (enviar msg, transição de estado).
    /// Antes da primeira tool consequencial, o dispatch verifica se chegou
    /// mensagem nova e reinicia o processamento se necessário.
    pub consequential: bool,
}

/// Resultado da execução de uma tool.
pub enum ToolResult {
    /// Tool executada com sucesso, retorna valor para o LLM.
    Ok(Value),
    /// Tool que transiciona o estado da máquina de estados.
    StateTransition { new_state: String, new_props: Value },
}

impl ToolDef {
    pub fn to_ollama_json(&self) -> Value {
        json!({
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": self.parameters,
            }
        })
    }
}

/// Tool global de finalização — sinaliza que o LLM terminou de agir.
pub fn done_tool() -> ToolDef {
    ToolDef {
        name: "done",
        description: "Chame esta ferramenta quando terminar de agir. Depois de enviar sua(s) mensagem(ns) ao cliente e salvar os dados necessários, chame done() para encerrar.",
        parameters: json!({
            "type": "object",
            "properties": {}
        }),
        consequential: false,
    }
}

/// Tool global disponível em todos os estados.
pub fn send_whatsapp_message_tool() -> ToolDef {
    ToolDef {
        name: "send_whatsapp_message",
        description: "Envia uma mensagem de texto para o cliente no WhatsApp. Esta é a ÚNICA forma de se comunicar com o cliente. Toda resposta deve ser enviada através desta ferramenta.",
        parameters: json!({
            "type": "object",
            "properties": {
                "message": {
                    "type": "string",
                    "description": "Texto da mensagem a enviar para o cliente"
                }
            },
            "required": ["message"]
        }),
        consequential: true,
    }
}

/// Consulta oficial se um CNPJ é optante pelo SIMEI.
pub fn consultar_simei_cnpj_tool() -> ToolDef {
    ToolDef {
        name: "consultar_simei_cnpj",
        description: "Consulta se um CNPJ é optante pelo SIMEI (ou seja, se é um MEI ativo). Use SEMPRE que o cliente informar um CNPJ, antes de aceitar que ele já tem MEI. A consulta leva 15-30 segundos. REGRA OBRIGATÓRIA DE USO: chame esta tool na MESMA resposta que o send_whatsapp_message de espera, em sequência, SEM done() entre as duas. Exemplo do fluxo correto numa única resposta: send_whatsapp_message('deixa eu dar uma olhada aqui rapidinho') → consultar_simei_cnpj(cnpj='...'). Se você chamar done() antes de consultar_simei_cnpj, a consulta nunca vai rodar e o cliente fica sem resposta. Não mencione 'Receita' ou 'portal' na mensagem de espera — diga só 'aqui'. Retorna optante_simei (bool), simei_desde, optante_simples, nome_empresarial. **OBRIGATÓRIO**: depois que esta tool retornar, você DEVE chamar send_whatsapp_message com uma resposta NOVA ao cliente baseada no resultado (ex: 'Confirmado! Vi que você é MEI desde X' ou 'Vi que esse CNPJ não é MEI'). A mensagem de espera anterior NÃO conta como resposta — ela só servia pra avisar que você ia consultar. Nunca termine o turno sem mandar uma mensagem nova contando o que descobriu.",
        parameters: json!({
            "type": "object",
            "properties": {
                "cnpj": {
                    "type": "string",
                    "description": "CNPJ a consultar. Pode vir com ou sem pontuação, a ferramenta normaliza."
                }
            },
            "required": ["cnpj"]
        }),
        consequential: true,
    }
}

/// Consulta se um código CNAE específico se qualifica para MEI.
pub fn consultar_cnae_por_codigo_tool() -> ToolDef {
    ToolDef {
        name: "consultar_cnae_por_codigo",
        description: "Verifica se um código CNAE específico está na lista de atividades permitidas para MEI. Use quando o cliente informar um código CNAE (ex: '4520-0/01') e quiser saber se pode ser MEI. Consulta rápida, sem mensagem de espera. Retorna pode_ser_mei (bool) e a lista de ocupações MEI que batem com o código. **OBRIGATÓRIO**: depois que esta tool retornar, você DEVE chamar send_whatsapp_message com uma resposta ao cliente baseada no resultado. Nunca termine o turno sem responder ao cliente.",
        parameters: json!({
            "type": "object",
            "properties": {
                "codigo": {
                    "type": "string",
                    "description": "Código CNAE, com ou sem formatação (ex: '4520-0/01' ou '4520001')"
                }
            },
            "required": ["codigo"]
        }),
        consequential: false,
    }
}

/// Consulta dívida ativa na lista de devedores da PGFN.
pub fn consultar_divida_pgfn_tool() -> ToolDef {
    ToolDef {
        name: "consultar_divida_pgfn",
        description: "Consulta se um CPF ou CNPJ possui dívida ativa na PGFN (Procuradoria-Geral da Fazenda Nacional). Use SEMPRE que o cliente informar um CPF ou CNPJ, para verificar se há débitos pendentes. A consulta leva 15-30 segundos. REGRA OBRIGATÓRIA DE USO: chame esta tool na MESMA resposta que o send_whatsapp_message de espera, em sequência, SEM done() entre as duas. Retorna tem_divida (bool), total_divida (valor em R$) e nome_devedor (se encontrado). **OBRIGATÓRIO**: depois que esta tool retornar, você DEVE chamar send_whatsapp_message com uma resposta baseada no resultado. Se tem_divida=true e total_divida > 15000, recuse o lead gentilmente com recusar_lead. Nunca mencione 'PGFN', 'dívida ativa' ou o valor exato pro cliente — diga apenas que identificou uma pendência cadastral.",
        parameters: json!({
            "type": "object",
            "properties": {
                "documento": {
                    "type": "string",
                    "description": "CPF (11 dígitos) ou CNPJ (14 dígitos). Pode vir com ou sem pontuação, a ferramenta normaliza."
                }
            },
            "required": ["documento"]
        }),
        consequential: true,
    }
}

/// Busca CNAEs MEI-compatíveis a partir de uma descrição de atividade.
pub fn buscar_cnae_por_atividade_tool() -> ToolDef {
    ToolDef {
        name: "buscar_cnae_por_atividade",
        description: "Busca CNAEs MEI-compatíveis a partir de uma descrição livre da atividade (ex: 'doces artesanais', 'cabelereiro', 'mecânico'). Use quando o cliente descrever o que faz mas não souber o código CNAE. Consulta rápida, sem mensagem de espera. Retorna até 10 ocupações MEI que batem com a descrição. **OBRIGATÓRIO**: depois que esta tool retornar, você DEVE chamar send_whatsapp_message com uma resposta ao cliente baseada no resultado. Nunca termine o turno sem responder ao cliente.",
        parameters: json!({
            "type": "object",
            "properties": {
                "descricao": {
                    "type": "string",
                    "description": "Descrição livre da atividade que o cliente exerce (ex: 'vendo bolo no pote', 'conserto celular')"
                }
            },
            "required": ["descricao"]
        }),
        consequential: false,
    }
}
