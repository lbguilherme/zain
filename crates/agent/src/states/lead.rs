use serde_json::{Value, json};

use crate::tools::{ToolDef, ToolResult};

use super::StateHandler;

pub struct LeadHandler;

impl StateHandler for LeadHandler {
    fn state_prompt(&self) -> String {
        r#"Você está conversando com uma pessoa que acabou de entrar em contato. Seu objetivo é:
1. Acolher a pessoa com simpatia e tirar dúvidas sobre o serviço
2. Entender a situação dela: já tem MEI? Qual atividade exerce?
3. Coletar informações progressivamente usando as ferramentas disponíveis
4. Quando tiver as informações necessárias, direcionar para pagamento com iniciar_pagamento()

- NÃO peça todas as informações de uma vez. Colete progressivamente conforme a conversa flui.
- Use as ferramentas para salvar dados assim que a pessoa fornecer.
- Quando souber se a pessoa já tem MEI (set_tem_mei) e tiver pelo menos nome e CPF, pode sugerir começar.
- Para iniciar_pagamento(), é necessário ter: nome, CPF, e saber se tem_mei.
- Se a pessoa já tem MEI, tente coletar o CNPJ também.
- Responda dúvidas sobre MEI, impostos, NF etc. com conhecimento — você é especialista."#
            .into()
    }

    fn tool_definitions(&self) -> Vec<ToolDef> {
        vec![
            ToolDef {
                name: "set_dados_pessoais",
                description: "Salva nome e/ou CPF do lead. Chame quando a pessoa informar esses dados.",
                consequential: false,
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "nome": {
                            "type": "string",
                            "description": "Nome completo da pessoa"
                        },
                        "cpf": {
                            "type": "string",
                            "description": "CPF (apenas números, 11 dígitos)"
                        }
                    }
                }),
            },
            ToolDef {
                name: "set_tem_mei",
                description: "Marca se a pessoa já possui MEI ou não.",
                consequential: false,
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "tem_mei": {
                            "type": "boolean",
                            "description": "true se já tem MEI, false se não tem"
                        }
                    },
                    "required": ["tem_mei"]
                }),
            },
            ToolDef {
                name: "set_cnpj",
                description: "Salva o CNPJ do MEI existente.",
                consequential: false,
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "cnpj": {
                            "type": "string",
                            "description": "CNPJ (apenas números, 14 dígitos)"
                        }
                    },
                    "required": ["cnpj"]
                }),
            },
            ToolDef {
                name: "set_atividade",
                description: "Salva a descrição da atividade e opcionalmente o CNAE.",
                consequential: false,
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "descricao": {
                            "type": "string",
                            "description": "Descrição da atividade (ex: 'vendo doces artesanais')"
                        },
                        "cnae": {
                            "type": "string",
                            "description": "Código CNAE, se conhecido"
                        }
                    },
                    "required": ["descricao"]
                }),
            },
            ToolDef {
                name: "set_endereco",
                description: "Salva o endereço do lead.",
                consequential: false,
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "endereco": {
                            "type": "string",
                            "description": "Endereço completo"
                        }
                    },
                    "required": ["endereco"]
                }),
            },
            ToolDef {
                name: "set_gov_br",
                description: "Salva as credenciais Gov.br do lead. Colete somente quando a pessoa fornecer voluntariamente.",
                consequential: false,
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "usuario": {
                            "type": "string",
                            "description": "Usuário Gov.br (geralmente CPF)"
                        },
                        "senha": {
                            "type": "string",
                            "description": "Senha Gov.br"
                        }
                    },
                    "required": ["usuario", "senha"]
                }),
            },
            ToolDef {
                name: "anotar",
                description: "Salva uma anotação livre sobre o cliente na memória. Use para registrar contexto relevante da conversa.",
                consequential: false,
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "texto": {
                            "type": "string",
                            "description": "Texto da anotação"
                        }
                    },
                    "required": ["texto"]
                }),
            },
            ToolDef {
                name: "iniciar_pagamento",
                description: "Inicia o fluxo de cadastro de cartão de crédito. Requer nome, CPF e saber se tem MEI.",
                consequential: true,
                parameters: json!({
                    "type": "object",
                    "properties": {}
                }),
            },
        ]
    }

    fn execute_tool(
        &self,
        name: &str,
        args: &Value,
        state_props: &mut Value,
        memory: &mut Value,
    ) -> ToolResult {
        match name {
            "set_dados_pessoais" => {
                if let Some(nome) = args.get("nome").and_then(|v| v.as_str()) {
                    state_props["nome"] = json!(nome);
                }
                if let Some(cpf) = args.get("cpf").and_then(|v| v.as_str()) {
                    state_props["cpf"] = json!(cpf);
                }
                ToolResult::Ok(json!({ "status": "ok", "dados_salvos": true }))
            }

            "set_tem_mei" => {
                if let Some(tem) = args.get("tem_mei").and_then(|v| v.as_bool()) {
                    state_props["tem_mei"] = json!(tem);
                }
                ToolResult::Ok(json!({ "status": "ok" }))
            }

            "set_cnpj" => {
                if let Some(cnpj) = args.get("cnpj").and_then(|v| v.as_str()) {
                    state_props["cnpj"] = json!(cnpj);
                }
                ToolResult::Ok(json!({ "status": "ok" }))
            }

            "set_atividade" => {
                if let Some(desc) = args.get("descricao").and_then(|v| v.as_str()) {
                    state_props["atividade_descricao"] = json!(desc);
                }
                if let Some(cnae) = args.get("cnae").and_then(|v| v.as_str()) {
                    state_props["cnae"] = json!(cnae);
                }
                ToolResult::Ok(json!({ "status": "ok" }))
            }

            "set_endereco" => {
                if let Some(end) = args.get("endereco").and_then(|v| v.as_str()) {
                    state_props["endereco"] = json!(end);
                }
                ToolResult::Ok(json!({ "status": "ok" }))
            }

            "set_gov_br" => {
                if let Some(usr) = args.get("usuario").and_then(|v| v.as_str()) {
                    state_props["gov_br_usuario"] = json!(usr);
                }
                if let Some(pwd) = args.get("senha").and_then(|v| v.as_str()) {
                    state_props["gov_br_senha"] = json!(pwd);
                }
                ToolResult::Ok(json!({ "status": "ok", "credenciais_salvas": true }))
            }

            "anotar" => {
                if let Some(texto) = args.get("texto").and_then(|v| v.as_str()) {
                    let existing = memory
                        .get("anotacoes")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let updated = if existing.is_empty() {
                        texto.to_owned()
                    } else {
                        format!("{existing}\n{texto}")
                    };
                    memory["anotacoes"] = json!(updated);
                }
                ToolResult::Ok(json!({ "status": "ok", "anotacao_salva": true }))
            }

            "iniciar_pagamento" => {
                let has_nome = state_props.get("nome").and_then(|v| v.as_str()).is_some();
                let has_cpf = state_props.get("cpf").and_then(|v| v.as_str()).is_some();
                let has_tem_mei = state_props
                    .get("tem_mei")
                    .and_then(|v| v.as_bool())
                    .is_some();

                if !has_nome || !has_cpf || !has_tem_mei {
                    return ToolResult::Ok(json!({
                        "status": "erro",
                        "mensagem": "Dados insuficientes. Necessário: nome, CPF e saber se tem MEI."
                    }));
                }

                let tem_mei = state_props["tem_mei"].as_bool().unwrap_or(false);

                ToolResult::StateTransition {
                    new_state: "COBRANCA".into(),
                    new_props: json!({
                        "motivo": "primeiro_pagamento",
                        "tem_mei": tem_mei,
                        "tentativas": 0,
                    }),
                }
            }

            _ => ToolResult::Ok(json!({
                "status": "erro",
                "mensagem": format!("Ferramenta '{name}' não reconhecida")
            })),
        }
    }
}
