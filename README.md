# Zain Gestão - Gestão de MEI no WhatsApp

## O que é

**Zain Gestão** é uma plataforma de gestão para MEI que funciona 100% pelo WhatsApp. O empreendedor resolve tudo por mensagem — sem portal do governo, sem app, sem burocracia.

**Primeiro mês grátis. Depois R$ 19,90/mês** — assinatura no cartão de crédito.

---

## Como funciona

### Jornada do usuário

1. Pessoa manda mensagem pro nosso número no WhatsApp
2. Entra como **lead** — a IA responde, tira dúvidas, entende o cenário
3. Cadastra cartão de crédito (primeiro mês grátis)
4. Se não tem MEI, a Zain abre pra ele. Se já tem, segue direto.
5. Vira **cliente** — a conversa continua, agora com todos os serviços ativos

Sem formulários, sem telas, sem fricção. Tudo acontece na mesma conversa.

---

## Serviços inclusos

| Serviço | Descrição |
|---|---|
| **Abertura de MEI** | Guia completo e execução do processo de abertura |
| **Baixa de MEI** | Encerramento do CNPJ quando necessário |
| **Emissão de nota fiscal** | NFS-e sob demanda, por texto ou áudio |
| **DAS mensal** | Geração e envio da guia de pagamento todo mês |
| **Lembrete e status de pagamento** | Alerta antes do vencimento + confirmação de pagamento |
| **Monitoramento do teto de faturamento** | Acompanhamento do acumulado no ano vs. limite de R$ 81k |
| **DASN-SIMEI anual** | Geração e envio da declaração anual do MEI |
| **Dúvidas contábeis e fiscais** | Respostas sobre impostos, obrigações, CNAE, etc. |

Tudo por **R$ 19,90/mês** (primeiro mês grátis).

---

## Arquitetura

```
WhatsApp (Usuário)
    |
    v
WhatsApp (whapi.cloud)
    |
    v
Backend Zain (Orquestrador)
    |
    +---> LLM Agente (Ollama / Gemma 4 27B) — conversa, decisões, orquestração de ferramentas
    |
    +---> Ferramentas do agente
    |     +---> Gov.br (login com credenciais do usuário)
    |     +---> PGMEI (guias DAS)
    |     +---> Portal NFS-e Nacional (emissão de nota)
    |     +---> Simples Nacional (DASN-SIMEI)
    |
    +---> Rotinas RPA — automação nos sites do governo
    |
    +---> Banco de Dados (clientes, empresas, histórico)
    |
    +---> Agendamentos (lembretes, DAS mensal, DASN anual)
```

Por trás é uma combinação de **LLM agente (Ollama/Gemma 4) com ferramentas**, **rotinas de RPA** nos portais do governo e **automações** para lembretes e gerações recorrentes.

---

## Autenticação

Precisamos das **credenciais Gov.br** (usuário e senha) do cliente para operar nos portais do governo em nome dele.

Isso é prática consolidada no mercado — MaisMei, Qipu, PJ Hero e outras plataformas já operam assim.

**Segurança:**
- Armazenamento em vault criptografado
- Consentimento explícito e documentado
- Criptografia em repouso e em trânsito
- Acesso auditado e logado

---

## Diferenciais

- **100% WhatsApp** — nenhum concorrente opera exclusivamente por WhatsApp como canal principal
- **R$ 19,90/mês** — concorrentes cobram R$ 49-195/mês
- **Texto livre e áudio** — sem formulários, sem telas
- **Proativo** — Zain inicia conversas (lembretes, alertas), não espera o usuário
- **Lead → Cliente na mesma conversa** — sem onboarding separado, sem cadastro em outro lugar
