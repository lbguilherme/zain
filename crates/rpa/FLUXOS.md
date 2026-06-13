# Fluxos de RPA do MEI — implementados e pendentes

Mapa das automações de portal do governo que a Zain faz (ou vai fazer) pelo
MEI. Cada "fluxo" é, no fundo, dirigir um portal via `chromium-driver` +
`crate::sanity`, com hCaptcha resolvido pela extensão NopeCHA.

Padrão de acesso dos portais:
- **Público por CNPJ** (PGMEI, DASN-SIMEI): só CNPJ + hCaptcha invisível, sem
  gov.br. Barato, sem depender de sessão.
- **Autenticado (gov.br)**: precisa da sessão gov.br do cliente (temos via
  `rpa::govbr::session` / `ensure_valid_session`).

Convenção de status abaixo: ✅ implementado · 🟡 parcialmente mapeado · 🔴 não
explorado.

---

## ✅ Já implementado

| Fluxo | Onde | Acesso |
|---|---|---|
| Login gov.br (+ 2FA) e captura de sessão | `rpa::govbr` / tools `auth_govbr*` | gov.br |
| Consulta CCMEI + elegibilidade MEI | `rpa::mei::certificado` / `refresh_mei_status` | gov.br |
| Abertura de MEI (inscrição) | `rpa::mei::inscricao` / tool `abrir_empresa` | gov.br |
| CCMEI inline (PDF) | tool `get_ccmei` | — (banco) |
| Consulta de dívida PGFN | `rpa::pgfn` | público |
| DAS mensal: consulta situação (histórico denso) | `rpa::pgmei::consultar_das_anos` / worker `das_refresh` | público CNPJ |
| DAS mensal: emitir guia (boleto + PIX) | `rpa::pgmei::emitir_das` / tool `emitir_das` | público CNPJ |
| DASN: consulta status anual (entregue/não) | `rpa::dasn::consultar_dasn` / worker `dasn_refresh` | público CNPJ |

---

## Pendências

### 1. 🟡 Preencher/transmitir a Declaração Anual (DASN-SIMEI)

A leitura já existe (`consultar_dasn`); falta o **preenchimento + transmissão**.

- **Sistema**: `dasnsimei.app` (Simples Nacional). **Público por CNPJ** +
  hCaptcha — sem gov.br. `rpa::dasn::identificar` já serve.
- **O que já mapeamos** (sonda `dasn_probe` com `DASN_PEEK=1`): wizard
  **Iniciar → Preencher → Resumo → Conclusão**.
  - Iniciar: seleciona o ano no radio `input[name=opcao][value=AAAA]`, clica
    `#iniciar-continuar`. O tipo (Original/Retificadora) é automático.
  - Preencher (**form mapeado**): `#input-rbt-icms` (receita de comércio e
    indústria — inclui transporte intermunicipal/interestadual e refeições),
    `#input-rbt-iss` (receita de serviços — locação e demais sem ICMS/ISS),
    receita bruta total (auto-soma, read-only), `#input-empregado-sim` /
    `#input-empregado-nao` (radio `name=informacao-empregado`),
    `#preencher-continuar`.
- **O que falta mapear**: **Resumo** e **Conclusão** — só aparecem
  transmitindo de verdade. Não dá pra mapear sem transmitir uma declaração
  real (não fazemos isso com CNPJ de terceiro). Precisa de um **cliente real
  consentindo** com declaração a entregar. É onde sai o **recibo (PDF)**.
- **Dados do cliente**: receita bruta do ano-calendário (idealmente separada
  comércio/indústria vs. serviços; o total é a soma), teve empregado no
  período (sim/não).
- **Riscos**: transmite declaração fiscal federal — ato com efeito legal,
  exige consentimento explícito. Entrega em atraso gera multa mínima de R$ 50.
  O portal mostra "Sistema SIMEI indisponível" em alguns casos (ex.: ano fora
  da vigência do MEI).
- **A fazer**: `rpa::dasn::declarar(cnpj, ano, receita_*, empregado)` + tool
  `declarar_dasn` (recibo PDF inline). O `identificar` já está estruturado pra
  encaixar.

### 2. 🔴 Dar baixa do MEI (encerramento do CNPJ)

- **Sistema**: Portal do Empreendedor (gov.br) — "Baixa do MEI".
  **Autenticado (gov.br)**.
- **O que sabemos**:
  - É o inverso da abertura (`rpa::mei::inscricao`); temos a sessão gov.br.
  - A baixa **exige a entrega de uma DASN de extinção** (declaração anual
    especial do ano da baixa) — conecta com o atributo
    `data-situacao-especial-eventobaixa` que aparece nos radios da DASN.
  - **Dívidas pendentes NÃO somem com a baixa** — continuam cobráveis (e podem
    ir pra dívida ativa). Importante deixar isso claro ao cliente.
- **Riscos**: **altíssimo** — encerra o CNPJ. Reabrir é possível mas
  burocrático. Exige consentimento forte e dupla confirmação. Não fazer por
  suposição.
- **A fazer**: explorar o portal de baixa (sonda), mapear o fluxo (incluindo a
  DASN de extinção), `rpa::mei::baixar` + tool.

### 3. 🔴 Consultar e fazer parcelamento de dívida

- **Dois sistemas**, conforme onde a dívida está:
  - **Parcelamento do Simples Nacional / MEI** (Receita) — para débitos de DAS
    **ainda não inscritos em dívida ativa**. gov.br **ou** código de acesso. É
    o "aplicativo de parcelamento" que o toast do PGMEI cita quando um mês está
    parcelado.
  - **PGFN** (`regularize.pgfn.gov.br`) — para débitos **já em dívida ativa da
    União**. gov.br.
- **O que já temos relacionado**: o `emitir_das` detecta `periodo_parcelado`;
  o estado `em_aberto` (anos anteriores) deliberadamente **não afirma
  "devedor"** justamente porque pode estar parcelado. Temos sessão gov.br.
- **Operações** (ordem valor × risco):
  1. **Consultar parcelamento ativo** (leitura, baixo risco): parcelas pagas /
     em aberto, saldo. **Resolve a ambiguidade do `em_aberto`/`periodo_parcelado`.**
  2. **Emitir o DAS da parcela** (geração): hoje só sabemos dizer "vá no app".
  3. **Formalizar um parcelamento** (escrita, ato legal): negocia a dívida em
     até 60 parcelas (mín. R$ 50/parcela). Consentimento explícito.
  4. Desistir do parcelamento.
- **Não explorado**: falta um CNPJ com parcelamento ativo **E** sessão gov.br
  no mesmo cliente (a IVONE `33.987.037/0001-04` tem parcelamento mas não é
  cliente).
- **A fazer**: explorar os dois portais; implementar ao menos **consultar
  status + emitir DAS da parcela** (casam direto com `em_aberto` /
  `periodo_parcelado`).

### 4. 🔴 Emitir nota fiscal

O fluxo **mais fragmentado** — não há um portal único.

- **NFS-e (serviços)**: é **municipal** (cada cidade tem seu sistema). Mas
  existe o **Emissor Nacional NFS-e** (padrão nacional, gov.br) que o MEI de
  serviços pode usar — **alvo único mais tratável** e cobre boa parte dos
  MEIs. Recomendação: **começar por aqui**.
- **NF-e / NFC-e (produtos/comércio)**: **estadual** (SEFAZ de cada estado).
  MEI que vende a consumidor final (PF) costuma ser dispensado; obrigatório em
  venda a PJ. Varia muito por estado.
- **NFA-e (avulsa)**: para quem não tem emissor próprio.
- **O que sabemos do cliente que ajuda**: o CNAE e a forma de atuação (sabemos
  se é serviço/comércio/indústria) permitem rotear NFS-e vs. NF-e.
- **Dados**: tomador (CPF/CNPJ, nome), descrição do serviço/produto, valor;
  para NFS-e, o código de serviço/atividade (o ISS do MEI é fixo no DAS, então
  a NFS-e geralmente sai sem ISS destacado, conforme regra do município).
- **Riscos**: documento fiscal com efeito legal — consentimento; e a
  fragmentação municipal/estadual é o maior obstáculo técnico.
- **A fazer**: decidir escopo (proposta: **Emissor Nacional NFS-e** primeiro,
  serviços), explorar, implementar.

### 5. 🔴 Cancelar nota fiscal

- **Sistema**: o mesmo emissor usado no #4 (Emissor Nacional NFS-e / SEFAZ).
- **O que sabemos**: cancelamento só dentro do prazo permitido — NFS-e segue a
  regra do município / Emissor Nacional; NF-e normalmente até ~24h, depois só
  carta de correção (não cancela). Depende de identificar a nota emitida.
- **A fazer**: junto com o emissor escolhido em #4 (depende dele).

---

## Outras operações/consultas que o MEI faz (candidatas)

Além das cinco acima, vale ter no radar (ordenado por utilidade aparente):

- **Alteração cadastral do MEI** — mudar atividades (CNAE), endereço, nome
  fantasia, forma de atuação. Portal do Empreendedor, gov.br. Operação comum
  (o agente já tem a skill `alteracao-cadastral-mei`, mas sem RPA).
- **Monitoramento do teto de faturamento (R$ 81k/ano)** — não é portal, é
  cálculo nosso a partir das NFs emitidas / receita declarada. Já é serviço
  anunciado no README.
- **Relatório Mensal de Receitas Brutas** — obrigação acessória do MEI
  (preenche mês a mês e guarda; não transmite). Poderíamos gerar/manter.
- **2ª via / comprovante de pagamento de DAS** — reimprimir guia paga ou puxar
  o comprovante.
- **Empregado do MEI** (1 permitido) — registro, folha, FGTS/INSS, eSocial.
  Agente já tem a skill `empregado-do-mei`.
- **Desenquadramento do SIMEI** — sair do MEI por ultrapassar o teto ou virar
  ME (diferente de baixa).
- **Certidão de regularidade / pendências (Receita + PGFN)** — consulta de
  débitos. Já temos parte (PGFN).
- **Contribuição previdenciária / extrato CNIS (Meu INSS)** — o DAS conta pra
  aposentadoria; cliente pode querer conferir tempo/contribuições.
- **Comprovação de renda** — declaração/extrato derivado da DASN/receita pra
  banco, financiamento, programas sociais.

---

## Notas transversais (valem pra qualquer fluxo novo)

- **hCaptcha**: portais do Simples (PGMEI/DASN) usam hCaptcha invisível
  resolvido pela NopeCHA (`launch::configure_nopecha`). Já validado.
- **Máscara de CNPJ**: os portais perdem as primeiras teclas se a gente digita
  antes da máscara anexar — usar o padrão de **digitar + reler + retentar** já
  presente em `pgmei`/`dasn`.
- **Toasts**: ler os toasts pós-ação (não confiar só no "deu certo"
  estrutural). O PGMEI ensinou que um sucesso pode vir junto de um aviso
  crítico (parcelamento) ou um erro (limite diário 23998).
- **Layout que muda por ano/contexto**: o PGMEI só mostra a coluna "Situação"
  no ano corrente — derivamos dos outros campos pra anos passados. Esperar
  variações assim em outros portais.
- **Limite diário de geração** (PGMEI): cachear same-day quando o resultado
  não muda no dia (ver `das_guia_cache`).
- **Atos com efeito legal** (declarar DASN, dar baixa, formalizar parcelamento,
  emitir/cancelar NF): exigem **consentimento explícito** do cliente e, em
  testes, **nunca** transmitir/efetivar com dados de terceiros.
