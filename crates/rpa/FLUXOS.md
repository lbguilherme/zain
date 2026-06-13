# Fluxos de RPA do MEI — implementados e pendentes

Mapa das automações de portal do governo que a Zain faz (ou vai fazer) pelo
MEI. Cada "fluxo" é, no fundo, dirigir um portal via `chromium-driver` +
`crate::sanity`, com hCaptcha resolvido pela extensão NopeCHA.

## Tipos de autenticação

- **Público** — só CNPJ + hCaptcha invisível, sem login (PGMEI, DASN-SIMEI).
  Barato, não depende de sessão.
- **gov.br** — precisa da sessão gov.br do cliente (temos via
  `rpa::govbr::session` / `ensure_valid_session`).
- **gov.br ou código de acesso** — alguns serviços da Receita aceitam o código
  de acesso do Simples Nacional como alternativa ao gov.br.
- **Certificado digital / login municipal** — emissão de NF estadual/municipal;
  varia por ente (o Emissor Nacional NFS-e, porém, usa gov.br).
- **— (interno)** — não tem portal; é cálculo/derivação nossa.

Status: ✅ implementado · 🟡 parcialmente mapeado · 🔴 não explorado.

---

## ✅ Já implementado

| Fluxo | Onde | Autenticação |
|---|---|---|
| Login gov.br (+ 2FA) e captura de sessão | `rpa::govbr` / tools `auth_govbr*` | gov.br |
| Consulta CCMEI + elegibilidade MEI | `rpa::mei::certificado` / `refresh_mei_status` | gov.br |
| Abertura de MEI (inscrição) | `rpa::mei::inscricao` / tool `abrir_empresa` | gov.br |
| CCMEI inline (PDF) | tool `get_ccmei` | — (banco) |
| Consulta de dívida PGFN | `rpa::pgfn` | público |
| DAS mensal: consulta situação (histórico denso) | `rpa::pgmei::consultar_das_anos` / `das_refresh` | público |
| DAS mensal: emitir guia (boleto + PIX) | `rpa::pgmei::emitir_das` / tool `emitir_das` | público |
| DASN: consulta status anual (entregue/não) | `rpa::dasn::consultar_dasn` / `dasn_refresh` | público |

---

## Pendências (ordenadas por complexidade crescente)

### 1. 🔴 Consultar regularidade / pendências fiscais (situação fiscal + CND) — **complexidade baixa**

- **Autenticação**: **público** (PGFN, já temos parcial) · **gov.br** (situação
  fiscal completa / CND no e-CAC).
- **O que sabemos**: já consultamos PGFN (`rpa::pgfn`). A visão completa
  (Receita + situação cadastral) sai do e-CAC autenticado.
- **A fazer**: consolidar "está regular?" cruzando PGFN + DAS em aberto;
  e-CAC só se precisarmos da CND oficial.

### 2. 🔴 Consultar contribuição previdenciária / CNIS — **complexidade baixa-média**

- **Autenticação**: **gov.br** (Meu INSS).
- **O que sabemos**: o DAS pago conta como contribuição pra aposentadoria; o
  cliente pode querer ver tempo/contribuições. É leitura.
- **A fazer**: explorar o Meu INSS (extrato CNIS) — leitura, sem efeito.

### 3. 🟡 Declarar / transmitir a DASN-SIMEI (declaração anual) — **complexidade média**

A leitura já existe (`consultar_dasn`); falta o **preenchimento + transmissão**.

- **Autenticação**: **público** (CNPJ + hCaptcha; `rpa::dasn::identificar` já serve).
- **O que já mapeamos** (sonda `dasn_probe` com `DASN_PEEK=1`): wizard
  **Iniciar → Preencher → Resumo → Conclusão**.
  - Iniciar: ano no radio `input[name=opcao][value=AAAA]` + `#iniciar-continuar`
    (tipo Original/Retificadora é automático).
  - Preencher (**form mapeado**): `#input-rbt-icms` (receita de comércio e
    indústria — inclui transporte intermunicipal/interestadual e refeições),
    `#input-rbt-iss` (receita de serviços — locação e demais sem ICMS/ISS),
    receita bruta total (auto, read-only), `#input-empregado-sim` /
    `#input-empregado-nao`, `#preencher-continuar`.
- **O que falta mapear**: **Resumo** e **Conclusão** (onde sai o **recibo PDF**)
  — só aparecem transmitindo de verdade. Precisa de um **cliente real
  consentindo** com declaração a entregar.
- **Dados do cliente**: receita bruta do ano (comércio/indústria vs. serviços),
  teve empregado (sim/não).
- **Riscos**: transmite declaração fiscal federal (ato legal) — consentimento
  explícito. Atraso = multa mínima R$ 50.
- **A fazer**: `rpa::dasn::declarar(...)` + tool `declarar_dasn` (recibo inline).

### 4. 🔴 Alteração cadastral do MEI — **complexidade média**

- **Autenticação**: **gov.br** (Portal do Empreendedor).
- **O que sabemos**: mudar atividades (CNAE), endereço, nome fantasia, forma de
  atuação. O agente já tem a skill `alteracao-cadastral-mei` (sem RPA). Temos os
  dados estruturados do cliente pra preencher.
- **Riscos**: muda o cadastro oficial — consentimento; confirmar antes.
- **A fazer**: explorar o fluxo de alteração no Portal do Empreendedor.

### 5. 🔴 Parcelamento de dívida (consultar → parcela → formalizar) — **complexidade média→alta**

Sub-operações que **escalam em complexidade/risco**:

- **Consultar parcelamento ativo** (leitura, baixo risco) — **resolve a
  ambiguidade do `em_aberto`/`periodo_parcelado`** que já existe no código.
- **Emitir o DAS da parcela** (geração) — hoje só sabemos dizer "vá no app".
- **Formalizar um parcelamento** (escrita, ato legal: até 60 parcelas, mín.
  R$ 50/parcela) — consentimento explícito.
- **Autenticação**: **gov.br ou código de acesso** (Parcelamento do Simples
  Nacional, para débito corrente) · **gov.br** (PGFN `regularize.pgfn.gov.br`,
  para dívida ativa). São **dois sistemas** conforme onde a dívida está.
- **Não explorado**: falta um CNPJ com parcelamento ativo **E** sessão gov.br
  (a IVONE `33.987.037/0001-04` tem parcelamento mas não é cliente).
- **A fazer**: explorar ambos; implementar pelo menos **consultar status +
  emitir DAS da parcela**.

### 6. 🔴 Desenquadramento do SIMEI — **complexidade alta**

- **Autenticação**: **gov.br** (Portal do Simples Nacional / Empreendedor).
- **O que sabemos**: sair do MEI por ultrapassar o teto (R$ 81k) ou virar ME —
  **diferente da baixa** (a empresa continua, muda de regime). Tem efeito
  tributário relevante (passa a ter contador, outros impostos).
- **Riscos**: alto — muda o regime da empresa. Consentimento + orientação clara.
- **A fazer**: explorar; provavelmente só executar com forte confirmação.

### 7. 🔴 Baixa do MEI (encerramento do CNPJ) — **complexidade alta**

- **Autenticação**: **gov.br** (Portal do Empreendedor).
- **O que sabemos**: inverso da abertura (`rpa::mei::inscricao`). **Exige a
  entrega de uma DASN de extinção** (declaração especial do ano da baixa —
  conecta com o `data-situacao-especial-eventobaixa` dos radios da DASN), então
  **depende do fluxo #4**. **Dívidas pendentes NÃO somem** com a baixa.
- **Riscos**: **altíssimo** — encerra o CNPJ. Reabrir é burocrático.
  Consentimento forte + dupla confirmação.
- **A fazer**: explorar o portal de baixa, mapear (incluindo a DASN de
  extinção), `rpa::mei::baixar` + tool.

### 8. 🔴 Empregado do MEI (eSocial) — **complexidade muito alta**

- **Autenticação**: **gov.br** (eSocial — sistema à parte).
- **O que sabemos**: o MEI pode ter **1 empregado**; envolve registro, folha,
  FGTS e INSS pelo eSocial (sistema separado e complexo). O agente já tem a
  skill `empregado-do-mei`.
- **Riscos**: obrigações trabalhistas com efeito legal recorrente.
- **A fazer**: avaliar viabilidade; é um subprojeto próprio.

### 9. 🔴 Emitir nota fiscal — **complexidade muito alta**

O fluxo **mais fragmentado** — não há um portal único.

- **Autenticação**: **varia**. NFS-e via **Emissor Nacional NFS-e** → **gov.br**
  (nacional, recomendado começar por aqui). NF-e/NFC-e (produtos) → **certificado
  digital / login da SEFAZ estadual**. NFS-e municipal "legada" → **login do
  município**.
- **O que sabemos**: NFS-e (serviços) é municipal, mas o **Emissor Nacional**
  unifica e cobre boa parte dos MEIs de serviço. NF-e (comércio/indústria) é
  estadual; MEI a consumidor final (PF) costuma ser dispensado, obrigatório a
  PJ. O CNAE/forma de atuação do cliente roteia serviço vs. produto.
- **Dados**: tomador (CPF/CNPJ, nome), descrição, valor; código de serviço
  (NFS-e). ISS do MEI é fixo no DAS → NFS-e geralmente sem ISS destacado.
- **Riscos**: documento fiscal (efeito legal) + fragmentação técnica enorme.
- **A fazer**: escopo (**Emissor Nacional NFS-e** primeiro), explorar, implementar.

### 10. 🔴 Cancelar nota fiscal — **complexidade muito alta**

- **Autenticação**: a mesma do #10 (depende do emissor usado).
- **O que sabemos**: só dentro do prazo permitido — NFS-e segue a regra do
  município / Emissor Nacional; NF-e normalmente até ~24h (depois só carta de
  correção). Depende de localizar a nota emitida.
- **A fazer**: junto com o emissor escolhido em #10 (depende dele).

---

## Operações internas (sem portal / sem RPA)

Não precisam de automação de portal — são cálculo/derivação nossa. **Autenticação: — (interno)**.

- **Monitoramento do teto de faturamento (R$ 81k/ano)** — acumula a receita
  (das NFs emitidas / DASN) e alerta ao se aproximar do limite. Já anunciado no
  README.
- **Relatório Mensal de Receitas Brutas** — obrigação acessória que o MEI
  preenche mês a mês e guarda (não transmite). Podemos gerar/manter por ele.
- **Comprovação de renda** — declaração/extrato derivado da DASN/receita pra
  banco, financiamento, programas sociais.

---

## Cadência das crons (recorrência por cliente)

Cada worker de background reconsulta cada cliente num intervalo próprio,
gravado em `<cron>_proxima_tentativa_em` (o worker só seleciona quem já
passou desse instante; `NULL` = nunca consultado, entra já). O intervalo é
uma **fórmula por cron** que combina a **situação do cliente** com um
**fator de atividade** — clientes inativos são espaçados pra bem longe (não
vale abrir portal/gov.br por quem sumiu).

**Fator de atividade** (de `last_activity_at`, carimbado a cada
`get_client_state`):

| última atividade | fator |
|---|---|
| ≤ 7 dias (ativo) | ×1 |
| 8–30 dias (morno) | ×2 |
| 31–90 dias (esfriando) | ×4 |
| > 90 dias ou nunca (inativo) | ×6 |

**Fórmulas:**

| Cron | Base (situação) | Intervalo |
|---|---|---|
| `mei_refresh` (gov.br) | MEI ativo confirmado: **30d**; sem MEI/elegibilidade: **7d** | `base × fator` (30d…180d / 7d…42d) |
| `das_refresh` (público) | âncora = menor vencimento `a_vencer` **+3d** (ou 24h) | `âncora + (fator−1)×14d` (ativo = no vencimento; inativo estica) |
| `dasn_refresh` (público) | **30d** (muda ~1x/ano) | `base × fator` (30d…180d) |

Notas:
- O **gov.br é o mais caro** (e desloga o cliente), então o `mei_refresh`
  com MEI confirmado fica em 30d×fator — a situação quase nunca muda.
- Falha transitória (portal instável) usa **backoff exponencial** na MESMA
  coluna `*_proxima_tentativa_em`, sobrepondo a cadência até o portal voltar.
- Eventos que mudam algo **na hora** não dependem da cron: o cliente
  declarando/pagando dispara `consultar_das`/`consultar_dasn`/`auth_govbr`
  interativos, que atualizam o estado imediatamente.

---

## Notas transversais (valem pra qualquer fluxo novo)

- **hCaptcha**: portais do Simples (PGMEI/DASN) usam hCaptcha invisível
  resolvido pela NopeCHA (`launch::configure_nopecha`). Já validado.
- **Máscara de CNPJ**: os portais perdem as primeiras teclas se a gente digita
  antes da máscara anexar — usar o padrão **digitar + reler + retentar** já
  presente em `pgmei`/`dasn`.
- **Toasts**: ler os toasts pós-ação (não confiar só no "deu certo"
  estrutural). O PGMEI ensinou que um sucesso pode vir junto de um aviso
  crítico (parcelamento) ou um erro (limite diário 23998).
- **Layout que muda por ano/contexto**: o PGMEI só mostra a coluna "Situação"
  no ano corrente — derivamos dos outros campos pra anos passados. Esperar
  variações assim em outros portais.
- **Limite diário de geração** (PGMEI): cachear same-day quando o resultado
  não muda no dia (ver `das_guia_cache`).
- **Atos com efeito legal** (declarar DASN, alterar cadastro, formalizar
  parcelamento, desenquadrar, dar baixa, emitir/cancelar NF): exigem
  **consentimento explícito** do cliente e, em testes, **nunca**
  transmitir/efetivar com dados de terceiros.
