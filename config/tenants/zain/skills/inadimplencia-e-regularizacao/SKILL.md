---
name: inadimplencia-e-regularizacao
description: "DAS atrasado e regularização: juros/multa, parcelamento (app/Portal e PGFN), transação/Refis, prescrição, intimações da Receita (Termo de Exclusão, MAED, DTE/e-CAC), dívida ativa, protesto e perda de benefícios. Use quando o MEI estiver com DAS em aberto, perguntar sobre juros, parcelar, negociar dívida, dívida ativa, intimação da Receita, ou se a dívida prescreve."
---
# DAS em atraso, dívida ativa e parcelamento

## Valores-base de 2026 (referência para cálculos)

Salário mínimo 2026: **R$ 1.621,00** (Decreto nº 12.797/2025, vigente desde 02/01/2026, quando o PGMEI passou a gerar DAS de competências de 2026). O INSS do DAS é 5% do salário mínimo; ICMS (R$ 1,00) e ISS (R$ 5,00) são fixos desde 2006.

| Atividade | INSS | ICMS | ISS | DAS mensal |
|---|---|---|---|---|
| Comércio ou Indústria | R$ 81,05 (5%) | R$ 1,00 | — | **R$ 82,05** |
| Serviços | R$ 81,05 (5%) | — | R$ 5,00 | **R$ 86,05** |
| Comércio e Serviços (misto) | R$ 81,05 (5%) | R$ 1,00 | R$ 5,00 | **R$ 87,05** |
| MEI Caminhoneiro | R$ 194,52 (12%) | varia | varia | **R$ 195,52 a R$ 200,52** (conforme ICMS/ISS) |

- Vencimento do DAS: **dia 20 de cada mês** (paga mesmo sem faturamento — é o que mantém a cobertura do INSS). Os valores de 2026 valem a partir da guia com vencimento em 20/02/2026 (competência janeiro/2026).
- O INSS reajusta sempre que o salário mínimo muda; ICMS (R$ 1,00) e ISS (R$ 5,00) seguem fixos. **Sempre confirme o valor do mês no PGMEI** — o sistema gera a guia com o valor já calculado.

## Autodiagnóstico de regularidade (faça ANTES de pagar ou parcelar)

Antes de gerar guia ou negociar, é preciso mapear **onde** está cada pendência — isso muda totalmente o local de pagamento. Há quatro frentes a checar:

| Pendência | Onde consultar | Quem cobra |
|---|---|---|
| DAS em atraso (ainda na Receita) | **PGMEI** / Portal do Simples Nacional → consulta de pendências | RFB (paga/parcela no próprio PGMEI) |
| Débitos já inscritos em dívida ativa | **Regularize (PGFN)** → consulta de inscrições; e **Lista de Devedores** da PGFN | PGFN (federal) / Estado / Município |
| DASN-Simei omissa + MAED (multa de declaração) | e-CAC → "Consulta Declaração Transmitida do MEI" | RFB (DARF à parte) |
| Comunicações, intimações e Termo de Exclusão | **DTE-SN** (Portal do Simples) **e** Caixa Postal do **e-CAC** | RFB / Estados / Municípios |

- O DAS é organizado **por competência (mês/ano)**. Pendência financeira (boleto) e pendência administrativa (DASN omissa) são coisas distintas — resolver só uma não regulariza o CNPJ.
- **Se houver tool de consulta de situação/DAS disponível, use a tool** para listar competências em aberto antes de orientar o cliente; não presuma o que está devendo.
- A partir do diagnóstico decide-se o caminho: pagar à vista no PGMEI, parcelar no Simei, ou — se já houve inscrição em dívida ativa — ir ao Regularize/PGFN e ao Estado/Município. Ver seções específicas abaixo.

## Como funcionam os encargos por atraso

Pago após o dia 20, o DAS recebe dois acréscimos, calculados automaticamente pelo PGMEI:

1. **Multa de mora:** 0,33% por dia de atraso, a partir do 1º dia após o vencimento, **limitada a 20%**. O teto de 20% é atingido em ~61 dias (20 ÷ 0,33 ≈ 60,6 dias). Depois disso a multa para de crescer.
2. **Juros de mora (Selic):** taxa Selic acumulada do mês seguinte ao vencimento até o mês anterior ao pagamento, **+ 1% no mês do pagamento**, incidindo sobre o valor do débito **sem a multa**. A Selic varia mês a mês; por isso o total muda a cada dia. Não afirme um valor fixo de juros — diga "o PGMEI calcula".

### Exemplo numérico (DAS de R$ 82,05 — comércio)
- Multa máxima (após ~61 dias): 20% × R$ 82,05 = **R$ 16,41**.
- A partir daí, só os juros Selic continuam acrescendo.
- Total exato sempre sai na guia atualizada do PGMEI.

**Pegadinha crítica:** nunca pagar boleto antigo já emitido. O valor de uma guia gerada semanas atrás está desatualizado; pagar não quita o débito, que continua aberto. Sempre gerar guia nova na hora.

## Passo a passo: regularizar DAS em atraso (PGMEI)

1. Acessar o **PGMEI** (Programa Gerador de DAS do MEI) no Portal do Simples Nacional.
2. Informar o CNPJ e clicar em Continuar.
3. Escolher **Emitir Guia de Pagamento (DAS)**.
4. Selecionar o **ano-calendário** do débito e marcar os **meses em atraso**.
5. Clicar em **Apurar/Gerar DAS** — sai a guia já com multa + juros atualizados.
6. Pagar (PIX, PDF/boleto na rede bancária ou pagamento online). Situação na Receita normaliza em **até 2 dias úteis**; o INSS costuma atualizar a carência no CNIS em **~30 dias** (prazo aproximado, sem fonte oficial fechada).

Para ver tudo em aberto antes de decidir pagar ou parcelar: consultar pendências dentro do PGMEI / Portal do Simples Nacional.

## Pagamento parcial/seletivo: por que NÃO regulariza o CNPJ

Pagar "só alguns meses" ou só os recentes deixando os antigos não devolve a regularidade — e cria armadilhas:

- **Cada DAS quita uma competência específica.** Pagar um boleto não "abate" outro mês. Enquanto restar qualquer competência em aberto, o CNPJ segue irregular (sem CND, sem certidão limpa).
- **Não basta pagar o boleto se a DASN-Simei do ano estiver omissa.** A omissão é pendência própria; é preciso entregar a declaração (que gera a MAED) **e** quitar/parcelar os DAS. Ordem prática: (1) diagnosticar pendências; (2) regularizar a parte administrativa (declarações); (3) quitar/negociar a parte financeira.
- **Ordem de imputação:** o MEI não escolhe livremente "pagar o mais novo e ignorar o velho". Cada guia é vinculada à sua competência, mas os débitos **mais antigos** são justamente os que primeiro vão para a dívida ativa — e, uma vez inscritos, **não podem mais ser pagos no PGMEI**. Por isso, priorizar as competências mais antigas reduz o risco de inscrição. Na lógica civil de imputação (arts. 352–355 do Código Civil), persistindo várias dívidas líquidas e vencidas, primeiro se quitam juros e as mais antigas — outra razão para não deixar o passivo antigo correr.
- **Boleto antigo pago "por baixo do valor real"** (guia desatualizada) gera quitação parcial: a competência fica em aberto pela diferença. Reemita sempre.
- **Diante de muitos meses em atraso, o parcelamento costuma ser mais seguro** que pagamentos parciais avulsos: ele consolida tudo e, após a 1ª parcela, suspende a exigibilidade e devolve a regularidade — sem deixar "lacunas".

## Parcelamento Simei (Receita Federal — débitos NÃO inscritos em dívida ativa)

Para débitos do DAS ainda na Receita (não enviados à PGFN).

- **Onde:** Portal do Simples Nacional ou Portal e-CAC da RFB → serviço "Parcelamento – Microempreendedor Individual". Há também o atalho gov.br "Parcelar imposto MEI".
- **Máx. de parcelas:** **60**.
- **Parcela mínima:** **R$ 50,00**.
- **Quantidade:** o sistema calcula automaticamente a maior quantidade possível (respeitando R$ 50 e o teto de 60); desde 2025 o MEI pode **escolher um número menor** de parcelas no momento do pedido.
- **Acréscimos no saldo parcelado:** Selic acumulada (do mês seguinte à consolidação até o mês anterior ao pagamento) **+ 1% no mês do pagamento**. Ou seja, juros continuam correndo durante o parcelamento — **à vista costuma sair mais barato**.
- **O que pode parcelar:** apenas débitos **já vencidos e declarados na DASN-Simei** na data do pedido.
- **Não entra:** débitos pagos por DARF, como a **multa por atraso na entrega da DASN (MAED)** — esses são pagos à parte.
- **Efetivação:** depende do **pagamento da 1ª parcela** até o vencimento. Se não pagar a 1ª, o pedido **não tem efeito** e o sistema libera nova solicitação. Pagar a 1ª parcela **suspende a exigibilidade** e devolve a regularidade.
- **Parcelas seguintes:** disponíveis a partir do **dia 10** dos meses posteriores; pagar até o **último dia útil** de cada mês.
- **Limite anual:** em regra, apenas **um pedido de parcelamento por ano-calendário**.
- **Desistência:** a qualquer tempo; encerra o parcelamento e os débitos não quitados seguem para cobrança / dívida ativa.
- **Rescisão (Receita):** falta de pagamento de **3 parcelas** (consecutivas ou não); ou saldo devedor após o vencimento da última parcela.
- **Atenção:** parcelamento **não tem desconto** sobre juros e multa — apenas dilui o pagamento. Desconto só existe em editais de **transação** (ver seção própria). Aderir a parcelamento **confessa a dívida** e, por isso, **interrompe a prescrição** (reinicia a contagem).

## Parcelamento x Transação ("Refis"): não confundir

São coisas diferentes — e a confusão leva o cliente a esperar um desconto que o parcelamento comum não dá.

| | **Parcelamento** (Simei ou PGFN convencional) | **Transação / Edital ("tipo Refis")** |
|---|---|---|
| Desconto | **Não há** — só dilui em parcelas | **Sim** — abate juros, multa e encargos (e, em editais específicos, parte do principal) |
| Disponibilidade | Permanente, contínua | **Por janela**: vale só enquanto o edital está aberto |
| Onde | PGMEI/Simei (não inscrito) ou Regularize (inscrito) | **Regularize/PGFN** (federais); programas próprios de Estado/Município |
| Efeito | Suspende exigibilidade após 1ª parcela | Idem, com os benefícios do edital |

- **Não existe "Refis do MEI" permanente.** "Refis" é apelido popular para programas/editais de transação que **abrem e fecham** em prazos definidos. Quando não há edital vigente aplicável ao caso, o único caminho é o parcelamento (sem desconto).
- **O agente orienta, mas NÃO adere pelo cliente.** A adesão a transação/parcelamento exige login gov.br nível **prata/ouro** e confissão da dívida — é decisão e ato do próprio MEI (ou de contador/procurador autorizado). O agente explica, encaminha ao Regularize e recomenda confirmar elegibilidade; não formaliza acordo.
- **Nunca prometa desconto sem confirmar o edital vigente** — percentuais, tetos e prazos mudam a cada edital.

### Edital de transação de pequeno valor — PGFN (referência: Edital nº 6/2026)

Exemplo concreto de edital aberto em 2026 (use como **referência do que costuma existir**, sempre confirmando o edital vigente no Regularize, pois condições e prazos variam a cada edição):

- **Regra específica MEI (código de receita 1537), dívida ≤ 5 salários mínimos:** **desconto de 50%** sobre o valor consolidado, em **até 60 prestações**. Parcela mínima MEI: **R$ 25,00** (demais contribuintes: **R$ 100,00**).
- **Transação de pequeno valor (PF, MEI, ME e EPP), inscrição ≤ 60 salários mínimos:** à vista, **50% de desconto**; no parcelado, **entrada de 5%** em até 5 prestações + saldo conforme o prazo: **até 7 meses → 50%**, **até 12 meses → 45%**, **até 30 meses → 40%**, **até 55 meses → 30%** de desconto.
- **Elegibilidade por data de inscrição** em dívida ativa da União (no Edital 6/2026: até **01/06/2025** para pequeno valor; até **03/03/2026** para as demais modalidades) — o que vale é o valor e a data de inscrição, não a capacidade de pagamento.
- **Adesão** exclusivamente pelo Regularize (SISPAR), até **30/09/2026, 19h** (prazo deste edital). Saldo corrigido pela **Selic + 1% no mês do pagamento**.
- **Rescisão** afasta os descontos, retoma a cobrança integral e **impede nova transação por 2 anos**.
- Estes números são do Edital 6/2026; **se não confirmar o edital vigente, trate percentuais/prazos como condicionais** e oriente a checar no Regularize.

## Inscrição em dívida ativa

Quando o MEI não paga nem parcela, os débitos **apurados nas DASN-Simei** podem ser inscritos em dívida ativa (possível desde out/2021). A cobrança **sai da Receita e vai para a PGFN** (federais) ou para Estado/Município.

| Tributo | Vai para | Acréscimo |
|---|---|---|
| INSS (previdenciário) + demais tributos federais | **PGFN** (Dívida Ativa da União) | **+20%** de encargo legal (Decreto-Lei 1.025/69) |
| ISS | **Município** | encargos da legislação municipal |
| ICMS | **Estado/DF** | encargos da legislação estadual |

A partir da inscrição, além de juros e multa, a dívida pode ser **protestada em cartório**, gera **CNPJ irregular** (sem CND — no máximo certidão positiva com efeitos de negativa se parcelada), dificulta crédito/financiamento e pode levar a penhora/bloqueio em execução fiscal.

## Prescrição e decadência: a dívida NÃO "some sozinha"

Pergunta recorrente: "se eu esperar 5 anos a dívida caduca?". A resposta honesta é **quase nunca na prática** — e contar com isso é arriscado.

- **Decadência (art. 173 do CTN):** prazo de **5 anos** que o Fisco tem para **constituir/lançar** o crédito. Esse prazo **não se interrompe nem se suspende** — corre corrido. No MEI, porém, o tributo é declarado pelo próprio contribuinte (DASN-Simei), então o crédito normalmente já nasce constituído; a discussão prática é quase sempre de **prescrição**, não decadência.
- **Prescrição (art. 174 do CTN):** prazo de **5 anos** para o Fisco **cobrar** o crédito já constituído, contado, em regra, do **vencimento** (lançamento por homologação) ou da constituição definitiva.
- **O que INTERROMPE a prescrição (zera e reinicia a contagem):** despacho do juiz que ordena a citação na execução fiscal; citação válida; protesto judicial; qualquer ato extrajudicial de reconhecimento do débito; e — atenção — **parcelamento ou pagamento parcial**, que confessam a dívida. Ou seja, **negociar reinicia o relógio**.
- **A inscrição em dívida ativa, por si só, NÃO interrompe nem suspende** a prescrição de dívida **tributária** (a suspensão de 180 dias da Lei 6.830/80 vale só para dívida não tributária, conforme o STJ). Mas, como a PGFN ajuíza execução e protesta, a prescrição "limpa" raramente se consuma.
- **Prescrição intercorrente:** se a execução fiscal já ajuizada fica paralisada por inércia do credor, há suspensão de 1 ano + 5 anos até a extinção — mecanismo processual, dependente de cada caso.
- **A dívida prescrita não desaparece automaticamente:** precisa ser **declarada/decretada** prescrita (de ofício pelo juiz ou via requerimento à PGFN para baixa). Enquanto não houver essa decretação, ela aparece como ativa, protestável e bloqueia certidões.
- **Postura do agente:** **não** orientar o cliente a "deixar prescrever". Avaliação de prescrição em caso concreto (datas, atos processuais, execução) é matéria de **contador/advogado tributarista** — encaminhar.

## Intimações e notificações da Receita: onde olhar e o que acontece se ignorar

Toda comunicação oficial ao MEI é **eletrônica** e tem **ciência presumida** — o prazo de defesa corre mesmo sem você ler.

**Onde consultar (monitore os dois canais):**
- **DTE-SN** (Domicílio Tributário Eletrônico do Simples Nacional e MEI) — no Portal do Simples Nacional. **Obrigatório e automático** para todo optante do Simples/MEI (LC 123/2006); não exige adesão. Concentra comunicações da RFB, Estados, Municípios e DF.
- **Caixa Postal do e-CAC** (DTE geral) — desde **jan/2026**, toda PJ inscrita no CNPJ passou a ter o DTE geral, então o MEI deve acompanhar **também** o e-CAC, não só o DTE-SN.
- Acesso por login gov.br (prata/ouro) ou código de acesso/certificado. Dá para cadastrar até 3 celulares e 3 e-mails para receber avisos de novas mensagens.

**Ciência presumida:** ocorre na **1ª leitura**, se acessar dentro de **45 dias** da disponibilização, **ou** automaticamente no **45º dia**, mesmo sem leitura. Se cair em dia não útil, conta no 1º dia útil seguinte.

**Tipos de comunicação:**

| Comunicação | O que é | Prazo / ação |
|---|---|---|
| **Cobrança amigável** | Aviso de débito antes de medida formal; chance de regularizar espontaneamente | Regularizar (pagar/parcelar/declarar) antes da inscrição em dívida ativa. Na MAED, entrega espontânea garante **50% de redução** |
| **Termo de Exclusão do Simples/Simei** | Intima o MEI a quitar pendências sob pena de desenquadramento | **90 dias** da ciência para regularizar a totalidade (prazo ampliado de 30 → 90 pela **LC 216/2025**); **20 dias úteis** da ciência para contestar ao Delegado de Julgamento. Não regularizar → exclusão/desenquadramento a partir de **01/01/2027** |
| **MAED** (Multa por Atraso na Entrega da Declaração) | Notificação de Lançamento gerada na hora ao transmitir a DASN-Simei em atraso | DARF emitido junto ao recibo; pagar em **30 dias** para manter o desconto de 50% da entrega espontânea |
| **Ação fiscal / MPF** (Mandado de Procedimento Fiscal) | Instrumento de fiscalização/auditoria formal (raro no MEI; diferente da MAED, que é automática) | Atender no prazo do termo; em fiscalização, **encaminhar a contador/advogado** |

**Se ignorar:** os prazos correm assim mesmo (ciência presumida em 45 dias). Resultado: multas e juros, perda de certidões, inscrição em dívida ativa, **exclusão do Simei** e até **CNPJ inapto**. Por isso a orientação padrão é **checar DTE-SN e e-CAC regularmente**.

## Como sair da dívida ativa (PGFN — portal Regularize)

Para débitos **já inscritos** na Dívida Ativa da União, o parcelamento/negociação NÃO é mais no Simples Nacional — é no **Regularize (PGFN)**.

1. Acessar **www.regularize.pgfn.gov.br** e login **gov.br nível prata ou ouro** (contas bronze não acessam mais).
2. Em **Negociação de dívida → Acessar o SISPAR**.
3. Menu **Adesão → Parcelamento → Avançar**.
4. Selecionar a negociação **0004 – Parcelamento Convencional** (ou modalidade de transação aplicável ao caso, ex.: edital de pequeno valor — ver seção de transação).
5. Selecionar os DAS a parcelar → **Confirmar → Sim**.
6. **Documento de Arrecadação** → emitir a **1ª parcela (DARF/DAS de prestação)**.

Regras PGFN:
- **Parcela mínima MEI:** varia por modalidade. Nos editais de **transação** recentes para MEI (ex.: Edital 6/2026) é **R$ 25,00** (demais contribuintes R$ 100,00); no **parcelamento convencional**, a referência histórica é **R$ 300,00**. **Verificar no SISPAR/Regularize o mínimo da modalidade escolhida** — não cravar valor sem confirmar.
- **Efetivação:** pagar a **1ª parcela (entrada) até o último dia útil do mês da adesão**; deferimento em **até 5 dias úteis** após esse pagamento. Não pagar implica indeferimento automático.
- **Guias seguintes:** disponíveis a partir do **dia 11** (após o dia 10); pagar lendo/digitando o código de barras (ou débito automático).
- **Portal Regularize:** seg–sex (exceto feriados nacionais), **07h–22h** (Brasília).
- **Rescisão:** falta de 3 prestações (consecutivas ou não); ou 1–2 prestações estando as demais quitadas; ou última prestação vencida e em aberto. Nas modalidades de transação, a rescisão também pode **bloquear nova transação por 2 anos**.
- Pago/parcelado, sai da lista de devedores ativos (fica visível como parcelado), restabelece certidões e suspende sanções.
- **Editais de transação de pequeno valor** abrem periodicamente com descontos sobre encargos — checar no Regularize se há edital vigente que beneficie o caso (não prometer desconto sem confirmar o edital).

## Prazos de atualização DEPOIS de quitar (atenção: dívida ativa é mais lenta)

Quitar não é "ficar regular na hora" — cada órgão tem seu prazo:

- **DAS no PGMEI (não inscrito):** Receita normaliza em **até 2 dias úteis**.
- **Dívida ativa na PGFN (pago por DARF/DAS de prestação):** o pagamento pode levar **até 5 dias úteis** para a rede bancária repassar e os sistemas da PGFN reconhecerem a baixa — **mais lento que o DAS comum**. Se passar disso e não baixar, pode ter havido erro de DARF (verificar imputação / usar Redarf na PGFN/RFB).
- **Protesto em cartório (se houve):** quitar/negociar **não cancela o protesto sozinho**. A PGFN envia a anuência eletrônica ao tabelionato em até **72h**, mas o cancelamento **só ocorre após o pagamento dos emolumentos/custas** diretamente no cartório ou via **CENPROT**. Ou seja: pagar a dívida + pagar as custas do cartório.
  - Se a CDA já foi enviada ao cartório mas o protesto **ainda não foi lavrado**, o pagamento para evitá-lo deve ser feito **no próprio cartório** (não na PGFN).
- **Negativação / restrição de crédito:** pode exigir baixa adicional além da quitação — orientar a acompanhar no Regularize e, se persistir, buscar o órgão competente.
- **INSS no CNIS:** carência reaparece em **~30 dias** (aproximado).

## Consequências da inadimplência (linha do tempo)

- **A cada mês em aberto:** acumula multa (até 20%) + juros Selic; perde regularidade fiscal.
- **12 meses de DAS sem pagar:** perde a **qualidade de segurado do INSS** (sem auxílio-doença, salário-maternidade, aposentadoria por invalidez); ao voltar a contribuir, **recomeça a carência**. Também enseja **exclusão do Simei** na apuração anual.
- **Termo de Exclusão** (RFB): após a ciência, **90 dias** para regularizar antes do desenquadramento do Simei (prazo ampliado de 30 → 90 dias pela LC 216/2025); **20 dias úteis** para contestar. Não regularizar → desenquadramento a partir de 01/01/2027 (na leva de 2026).
- **DASN-Simei não entregue por 2 anos consecutivos:** caminho para CNPJ **inapto** (impede emitir NF, abrir conta PJ, licitações) e suspensão.
- **Suspensão do CNPJ → cancelamento (baixa):** após suspensão, há prazo para regularizar (≈95 dias, reduzido para 30 dias se já houver Termo de Exclusão — prazos aproximados, sem fonte oficial fechada); persistindo, **baixa definitiva e irreversível**. O CNPJ baixado exige nova formalização; **dívidas migram para o CPF do titular** e podem ser cobradas por até **5 anos** a partir do último mês antes do cancelamento.

## Restituição (DAS pago a mais / em duplicidade / indevido)

- **INSS:** app **Pedido Eletrônico de Restituição**, no Portal do Simples Nacional. **Não há atendimento presencial** — tudo eletrônico.
- **ICMS:** junto ao **Estado/DF**. **ISS:** junto ao **Município**.
- **Não há compensação** a pedido para o MEI: se pagou em duplicidade/a maior, o caminho é **pedir restituição** (não abater do mês seguinte).
- **Prazo para pedir:** até **5 anos** da data do pagamento.
- **Conta bancária:** obrigatória — PJ (do CNPJ) ou PF (do CPF do responsável); corrente, pagamento, poupança ou PIX.
- **Prazo de pagamento da restituição:** em casos regulares, **~60 dias**.

## Erros comuns e pegadinhas

- Pagar boleto antigo/desatualizado → débito continua aberto. Gere guia nova.
- Achar que parcelamento "resolve sozinho": só vale após **pagar a 1ª parcela** no prazo.
- Confundir parcelamento (sem desconto) com transação/edital "Refis" (com desconto): só a transação dá abatimento, e só enquanto o edital está aberto. **Não existe "Refis do MEI" permanente.**
- Contar que "a dívida prescreve em 5 anos e some": parcelar/pagar parcial **interrompe** a prescrição, execução/protesto também — e a dívida prescrita ainda precisa ser **decretada**. Não some sozinha.
- Tentar parcelar dívida ativa pelo Simples Nacional → tem que ser no **Regularize/PGFN**.
- Pagar a dívida protestada e achar que o protesto cai sozinho: **falta pagar as custas do cartório** (direto no tabelionato ou via CENPROT).
- Esquecer ISS/ICMS em atraso: a parte municipal/estadual da dívida ativa é cobrada pelo **Município/Estado**, não pela PGFN — pode exigir regularização separada.
- Confundir não pagar DAS com não entregar DASN: são pendências distintas, ambas levam a sanções; muitas vezes é preciso resolver as duas (declarar + pagar/parcelar).
- Ignorar o DTE-SN / e-CAC: a **ciência é presumida em 45 dias** mesmo sem leitura; prazos de Termo de Exclusão e defesa correm assim mesmo.
- Pagar só os meses recentes e deixar os antigos: os **antigos** vão para dívida ativa e saem do PGMEI; o CNPJ segue irregular.
- Supor benefício do INSS imediato após quitar: a carência reaparece no CNIS em ~30 dias, mas 12 meses sem pagar já podem ter feito perder a qualidade de segurado.

## Quando o agente deve encaminhar, recusar ou consultar tool

- **Diagnóstico de pendências, gerar/consultar DAS, valor exato do mês, multa e juros:** orientar a usar o **PGMEI** e, se houver tool de consulta/geração de DAS disponível, **consultar a tool** em vez de calcular à mão. Nunca cravar valor de juros Selic de memória.
- **Dívida já inscrita em dívida ativa:** direcionar para o **Regularize/PGFN** (federais) e **Estado/Município** (ICMS/ISS). Não prometer descontos sem confirmar edital vigente.
- **Transação / "Refis" / editais de desconto:** explicar a diferença para parcelamento, indicar que percentuais e prazos variam por edital e exigem confirmação no Regularize. **O agente orienta, mas NÃO adere pelo cliente** — a adesão é ato do MEI/contador (login prata/ouro, confissão da dívida).
- **Parcela mínima na PGFN, descontos de transação, elegibilidade de edital:** valores variam por modalidade — orientar a confirmar no SISPAR/Regularize.
- **Prescrição/decadência em caso concreto, contestação de Termo de Exclusão, defesa em ação fiscal/MPF, cancelamento/baixa já consumado, execução fiscal, penhora, protesto:** orientar busca de regularização e **encaminhar a contador/jurídico**; o agente não conduz defesa, não avalia prescrição de caso concreto nem instrui processo.
- **Restituição de ICMS/ISS:** encaminhar ao Estado/Município (a tool/Receita só trata o INSS).
- **Sempre que variar por município/estado/UF** (ISS, ICMS, protesto estadual, prazos de procuradorias estaduais): dizer explicitamente que depende do ente e orientar a confirmar no órgão local.

## Fontes

- Simples Nacional – Atualização de valores devidos em 2026: https://www8.receita.fazenda.gov.br/simplesnacional/Noticias/NoticiaCompleta.aspx?id=c3b2044c-ff97-432a-b33c-ecf2a3df6dc3
- Manual do Parcelamento de Débitos do MEI (RFB/Simples Nacional): https://www8.receita.fazenda.gov.br/SimplesNacional/Arquivos/manual/Manual_Parcelamento_MEI.pdf
- Gov.br – Parcelar imposto do MEI: https://www.gov.br/pt-br/servicos/parcelar-imposto-mei
- PGFN – Portal Regularize: https://www.regularize.pgfn.gov.br/
- PGFN/SISPAR – Parcelamento da dívida ativa: https://sisparnet.pgfn.fazenda.gov.br/
- PGFN – Transação de pequeno valor, Edital nº 6/2026: https://www.gov.br/pgfn/pt-br/servicos/orientacoes-contribuintes/acordo-de-transacao/edital-no-6-2026/transacao-de-pequeno-valor-edital-ndeg-06-2026
- PGFN – Protesto da Dívida Ativa da União (como proceder / cancelamento): https://www.gov.br/pgfn/pt-br/servicos/orientacoes-contribuintes/protesto-de-certidao-da-divida-ativa-da-uniao/como-proceder
- PGFN – Decadência e prescrição: https://www.gov.br/pgfn/pt-br/cidadania-tributaria/por-assunto/cobranca-e-restituicao-1/decadencia-e-prescricao/prescricao
- PGFN – Lista de Devedores: https://www.listadevedores.pgfn.gov.br/
- Planalto – Lei nº 5.172/1966 (CTN), arts. 173 e 174 (decadência e prescrição): https://www.planalto.gov.br/ccivil_03/leis/l5172compilado.htm
- Planalto – Decreto-Lei nº 1.025/1969 (encargo legal de 20%): https://www.planalto.gov.br/ccivil_03/decreto-lei/del1025.htm
- Receita Federal – Termo de Exclusão para devedores do Simples Nacional, incluindo MEI (2026; prazo de 90 dias, LC 216/2025): https://www.gov.br/receitafederal/pt-br/assuntos/noticias/2026/marco/receita-federal-emite-termo-de-exclusao-para-devedores-do-simples-nacional-incluindo-mei
- Simples Nacional – Domicílio Tributário Eletrônico do Simples Nacional e MEI (DTE-SN): https://www8.receita.fazenda.gov.br/SimplesNacional/Servicos/Grupo.aspx?grp=20&id=66
- Manual do DTE-SN (RFB): https://www8.receita.fazenda.gov.br/SimplesNacional/Arquivos/manual/Manual_DTE.pdf
- Gov.br – Consultar comunicações do Simples Nacional (DTE-SN): https://www.gov.br/pt-br/servicos/consultar-comunicacoes-do-simples-nacional
- Sebrae – Como reimprimir a multa por atraso na entrega da DASN-Simei (MAED): https://sebrae.com.br/sites/PortalSebrae/artigos/como-reimprimir-a-multa-por-atraso-na-entrega-da-dasn-simei,983700c25828b510VgnVCM1000004c00210aRCRD
- Sebrae – Suspensão e cancelamento de inscrição do MEI inadimplente: https://sebrae.com.br/sites/PortalSebrae/ufs/ce/sebraeaz/suspensao-e-cancelamento-de-inscricao-do-mei-inadimplente,60f67314282c0610VgnVCM1000004c00210aRCRD
- Decreto nº 12.797/2025 – salário mínimo 2026 (R$ 1.621,00): https://www.planalto.gov.br/ccivil_03/_ato2023-2026/2025/decreto/d12797.htm
- Lei Complementar nº 216/2025 (ampliação do prazo do Termo de Exclusão para 90 dias): https://www.planalto.gov.br/ccivil_03/leis/lcp/lcp216.htm
