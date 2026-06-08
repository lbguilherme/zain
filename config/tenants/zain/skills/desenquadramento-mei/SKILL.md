---
name: desenquadramento-mei
description: "Sair ou ser excluído do MEI: excesso de faturamento (até 20% x acima de 20%, retroativo), atividade vedada, 2º empregado, filial, virar sócio; comunicação no SIMEI, virada para ME, planejamento do teto, receita de marketplace/PIX e risco de fracionar em vários MEIs. Use quando disserem que passaram do limite ou faturaram mais de 81 mil, que vão estourar, que foram desenquadrados ou viraram ME, que vendem em marketplace, ou perguntarem o custo de virar ME."
---
# Desenquadramento do MEI

## Conceito

Desenquadramento é deixar de cumprir qualquer requisito do regime SIMEI (Sistema de Recolhimento em Valores Fixos Mensais do MEI). **Não é a baixa do CNPJ:** o CNPJ continua ativo, mas a empresa deixa de recolher o DAS fixo e passa a ser **Microempresa (ME)** ou **Empresa de Pequeno Porte (EPP)** tributada pela regra geral do Simples Nacional. Pode ocorrer **por opção** (o empreendedor decide sair), por **comunicação obrigatória** (incorreu em vedação e precisa avisar) ou **de ofício** (a Receita identifica e desenquadra automaticamente).

Base legal: LC 123/2006 (em especial art. 18-A; art. 36-A para a multa) e Resolução CGSN nº 140/2018. O limite anual de R$ 81.000 está em vigor desde 2018.

## Motivos de desenquadramento

| Motivo | Categoria | Efeito (quando comunicado no prazo) |
|---|---|---|
| Receita bruta anual acima de R$ 81.000 **em até 20%** (≤ R$ 97.200) | Obrigatório | A partir de 1º/jan do **ano seguinte** |
| Receita bruta anual **acima de 20%** (> R$ 97.200) | Obrigatório | **Retroativo a 1º/jan do ano** do excesso (ou à data de abertura, se 1º ano) |
| Exercer atividade (CNAE) não permitida ao MEI (fora da lista de ocupações do Anexo XI da Res. CGSN 140/2018) | Obrigatório / automático | A partir do mês seguinte ao do evento |
| Contratar mais de 1 empregado | Obrigatório | A partir do mês seguinte ao do evento |
| Pagar ao único empregado mais de 1 salário mínimo ou o piso da categoria | Obrigatório | A partir do mês seguinte ao do evento |
| Abrir filial | Obrigatório / automático | A partir do mês seguinte ao do evento |
| Tornar-se titular, sócio ou administrador de outra empresa | Obrigatório | A partir do mês seguinte ao do evento |
| Alterar a natureza jurídica (deixar de ser empresário individual; entrada de sócio; virar S/A) | Automático | A partir do mês seguinte ao do evento |
| Incorrer em qualquer vedação de exclusão do próprio Simples Nacional | Obrigatório | Conforme o caso |

**Comunicação por equivalência (automática):** alterar para natureza jurídica distinta de empresário individual, incluir atividade não permitida ao MEI ou abrir filial via alteração cadastral no CNPJ **já equivale** à comunicação obrigatória — o sistema desenquadra sozinho, sem passo manual separado.

### Limite proporcional no ano de abertura

No ano em que o CNPJ é aberto, o teto é proporcional aos meses de atividade: R$ 6.750 por mês (incluindo o mês de abertura). Ex.: aberto em julho → 6 meses → teto de R$ 40.500 até dezembro. A regra dos 20% incide sobre o limite **proporcional**, não sobre os R$ 81.000 cheios.

## A regra dos 20% (excesso de faturamento) — o ponto mais sensível

Pontos de corte (2026, teto de R$ 81.000):

- **20% acima do teto = R$ 97.200.**
- Faturou **até R$ 97.200** → excesso de até 20% (Cenário A).
- Faturou **acima de R$ 97.200** → excesso superior a 20% (Cenário B).

### Cenário A — excesso de ATÉ 20% (R$ 81.000,01 a R$ 97.200)

- Continua MEI normalmente até **31/dez**, recolhendo o DAS fixo.
- A migração para ME ocorre só em **1º de janeiro do ano seguinte** (sem retroatividade).
- Na **DASN-SIMEI** (declaração anual, entregue até 31/maio do ano seguinte) o sistema gera um **DAS complementar** sobre a parcela que excedeu R$ 81.000, tributada pela tabela do Simples Nacional do anexo correspondente.
- **Exemplo (comércio):** faturou R$ 95.000 → excedente = R$ 95.000 − R$ 81.000 = R$ 14.000. O DAS complementar incide sobre esses R$ 14.000 pela alíquota do Anexo I. A alíquota **efetiva** depende do RBT12 e do CNAE; não afirme valor fechado — confirme o cálculo no PGMEI/DASN-SIMEI ou com contador.

### Cenário B — excesso SUPERIOR a 20% (acima de R$ 97.200)

- Desenquadramento **retroativo** a 1º/jan do ano-calendário do excesso. Se for o ano de abertura, retroage **à data de abertura do CNPJ**.
- Efeito prático: é como se a empresa **nunca tivesse sido MEI** naquele ano. Todos os tributos do período são **recalculados pela regra do Simples Nacional como ME**, com **juros (Selic) e multa** sobre o atraso. Os DAS fixos já pagos como MEI são abatidos do montante devido.
- **Prazo de comunicação obrigatória:** até o **último dia útil do mês subsequente** àquele em que o faturamento ultrapassou os 20% (não esperar a declaração anual).

> Pegadinha frequente: o cliente acha que "só paga sobre o que passou". Isso **só vale no Cenário A**. No Cenário B o recálculo é sobre **toda a receita do ano** pela tabela do Simples, não apenas sobre o excedente.

### O que conta como receita bruta

Tudo o que entra pela atividade da empresa, em **qualquer forma de recebimento**: dinheiro, PIX, cartão de débito/crédito, boleto, TED/DOC, recebimentos em marketplaces. A receita é o **valor bruto da venda**, antes de descontar taxas de plataforma, comissões, frete, anúncios ou embalagem — essas são despesas e **não reduzem** o faturamento (detalhado na seção "Marketplaces, PIX no CPF e cruzamento de dados"). A Receita cruza notas fiscais, maquininhas, e-Financeira e marketplaces. A Resolução CGSN nº 183/2025 reforçou o critério de que rendimentos da **mesma atividade econômica** recebidos no CPF contam para o limite do CNPJ — a própria Receita esclareceu que isso não significa somar CPF + CNPJ de forma automática (ver "PIX no CPF"). Não há como "esconder" faturamento.

## Planejamento preventivo: quem está faturando acima do ritmo do teto

Esta é a situação de quem **ainda não estourou**, mas vem faturando rápido e quer se antecipar. O objetivo é evitar cair no Cenário B (excesso > 20%, retroativo e caro) e chegar à virada para ME organizado.

### Em quantos meses o teto estoura (estimativa de ritmo)

Faça a conta com o faturamento acumulado no ano-calendário (jan a dez), não em janela móvel:

| Faturamento médio/mês | Projeção anual (×12) | Situação no fim do ano |
|---|---|---|
| até R$ 6.750 | até R$ 81.000 | dentro do teto |
| ~R$ 7.000 a R$ 8.100 | R$ 84.000 a R$ 97.200 | excesso de até 20% (Cenário A) |
| acima de R$ 8.100 | acima de R$ 97.200 | excesso > 20% (Cenário B, retroativo) |

Regra prática para o cliente: **R$ 6.750/mês** é o ritmo do teto; **R$ 8.100/mês** é o ritmo do limite dos 20%. Quem ultrapassa R$ 8.100/mês em média caminha para o Cenário B.

Para estimar quando estoura: `meses_até_estourar ≈ (81.000 − faturamento_acumulado_no_ano) ÷ faturamento_médio_mensal`. Ex.: já faturou R$ 50.000 e fatura ~R$ 9.000/mês → faltam ~R$ 31.000 → estoura em ~3,4 meses; nesse ritmo o ano fecharia bem acima de R$ 97.200 (Cenário B). Esse é um sinal para agir **antes** de dezembro.

### Fluxo proativo recomendado

1. **Monitorar o acumulado mensalmente** (some todas as entradas da atividade: CNPJ + PIX no CPF da mesma atividade + marketplaces + maquininha).
2. **Se a projeção apontar Cenário A (até R$ 97.200):** pode ser estratégico **segurar/escalonar** vendas no fim do ano para fechar mais perto de R$ 81.000 e reduzir o DAS complementar — mas só se o faturamento for controlável (ex.: adiar entregas para janeiro do ano seguinte, que já é outro ano-calendário). Nunca deixar de declarar.
3. **Se a projeção apontar Cenário B (> R$ 97.200):** o melhor caminho costuma ser **planejar a migração para ME**, contratar contador com antecedência e estruturar emissão de notas, controle de receita e reserva de caixa para os tributos variáveis — não esperar a cobrança retroativa.
4. **Organizar contador antes da virada**, não depois: a ME exige escrituração e obrigações acessórias mensais desde o início (ver seção de custo).

> **Não existe desenquadramento "preventivo" no sistema:** o MEI só **comunica** o desenquadramento depois de efetivamente incorrer na vedação. "Preventivo" aqui significa **planejamento** (contador, caixa, projeção), não um pedido antecipado no portal. Se o cliente quiser sair antes de estourar, o caminho é o **desenquadramento por opção** (ver prazos no quadro do SIMEI).

### Faturamento pontual / não recorrente

Um pico isolado de faturamento (uma venda grande, um mês atípico) **não isenta** do desenquadramento: o que vale é o **total do ano-calendário**, independentemente de a receita ser recorrente ou pontual. Se o acumulado do ano passar de R$ 81.000, há desenquadramento; se passar de R$ 97.200, é Cenário B retroativo, mesmo que o excesso tenha vindo de um único mês.

Porém, quando o excesso fica **dentro de até 20%** (Cenário A), o efeito é só no ano seguinte e **dá para reenquadrar** em janeiro do ano seguinte, voltando a ser MEI, **desde que o novo ano-calendário volte a respeitar o teto**. Já no **Cenário B (> 20%)** não dá para voltar a ser MEI no mesmo ano — segue como ME (ver "Como voltar a ser MEI"). Ou seja: um estouro pontual de até 20% é, na prática, uma "saída temporária" reversível; acima de 20% é uma saída efetiva no ano.

## Custo de manter ME vs. MEI (para a decisão de migrar)

Comparação de **ordem de grandeza** para o cliente dimensionar a diferença. Os valores de honorários **não são tabelados** (são livres e variam por escritório, cidade, anexo, volume de notas e número de empregados); confirme orçamento real com contador.

| Item | MEI | ME no Simples Nacional |
|---|---|---|
| Tributo mensal | **DAS fixo** (R$ 82,05 a R$ 87,05 em 2026, conforme atividade) | **DAS variável** sobre o faturamento (faixa típica de ~4% a ~12% conforme anexo e RBT12) |
| Contador | **Opcional** (regime dispensa contabilidade) | **Obrigatório** por lei (escrituração + obrigações acessórias mensais) |
| Honorário contábil mensal (ordem de grandeza) | ~R$ 0 a R$ 350 (se contratar contabilidade online) | **~R$ 137 a R$ 350/mês** em contabilidade online básica; **~R$ 400 a R$ 1.200+/mês** em escritório tradicional ou casos complexos (comércio, muitas notas, empregados) |
| Complexidade | Baixíssima | Média/alta (apuração mensal do DAS, segregação de receitas, ICMS/ISS) |

Leitura para o cliente: ao virar ME, o custo fixo do contador (ordem de **algumas centenas de reais por mês**) entra como despesa nova e recorrente, **somado** ao DAS que passa a ser variável e proporcional ao faturamento. Para faturamentos pouco acima do teto, é comum a carga total subir de forma relevante frente ao MEI — por isso vale o cálculo caso a caso com contador antes de decidir entre escalar (virar ME) ou conter o faturamento dentro do teto.

> A Reforma Tributária (IBS/CBS) está em transição e tende a aumentar a complexidade da apuração da ME a partir de 2026 (emissão de notas vinculada a PIX/cartão), o que reforça a necessidade de contabilidade especializada. Não afirme alíquotas ou regras finais da reforma sem confirmar — remeta ao contador.

## Virou ME no Simples Nacional: o que muda

- **Tributação deixa de ser fixa.** Em vez do DAS fixo, paga-se DAS variável conforme faturamento e anexo: passam a incidir IRPJ, CSLL, PIS, COFINS, CPP (contribuição patronal), além de ICMS (comércio/indústria) e/ou ISS (serviços), mais IPI para indústria.
- **Anexos:** Anexo I (comércio, alíquotas nominais de 4% a 19%), Anexo III (serviços, 6% a 33%), Anexos IV e V conforme a atividade. Serviços podem cair no Anexo III (mais barato) ou V (mais caro) conforme o **Fator R** (folha/receita ≥ 28% → Anexo III). A alíquota **efetiva** é menor que a nominal (fórmula do RBT12).
- **Contador passa a ser obrigatório** (Código Civil / escrituração contábil). Ao contrário do MEI, a ME precisa de contabilidade e de entrega de obrigações acessórias mensais; sem isso a empresa pode ser inativada.
- **Limites de porte (2026):** ME até R$ 360.000/ano; EPP de R$ 360.000 a R$ 4.800.000/ano. O porte de destino depende do faturamento.

### Atividade mista (produto + serviço) na virada para ME: segregação por anexo

Se o cliente vende mercadoria **e** presta serviço (ex.: pet shop com loja + banho/tosa; oficina com peças + mão de obra; salão com venda de cosméticos + serviço), na ME **não se soma tudo numa alíquota só**. A LC 123/2006, art. 18, §4º, obriga a **segregação das receitas** por natureza:

- Receita de **revenda de mercadoria/produto** → tributada pelo **Anexo I** (comércio).
- Receita de **prestação de serviço** → tributada pelo **Anexo III, IV ou V** conforme a atividade (Fator R decide entre III e V em vários serviços).
- A segregação é feita no **PGDAS-D** e deve refletir a natureza de cada **nota fiscal** desde a emissão (descrição correta de produto x serviço).
- O **RBT12** que define a faixa de alíquota é o **total combinado** das duas atividades; o **DAS final é a soma** dos cálculos de cada anexo, recolhido em **uma única guia**.
- Comércio puro (Anexo I) **não paga ISS**; o ISS (municipal) só aparece nos Anexos III/IV/V. Logo a parcela de serviço pode carregar ISS — alíquota e regras **variam por município**.

Errar a segregação (lançar tudo como comércio, por ser mais barato) é falha fiscal grave e facilmente detectada. O cálculo do DAS misto é tarefa de **contador**.

## Alterar atividade (CNAE): alteração cadastral x desenquadramento

Distinção que o cliente costuma confundir:

| Situação | É o quê | Onde / como |
|---|---|---|
| Novo CNAE **consta** na lista de ocupações do MEI (Anexo XI da Res. CGSN 140/2018) | **Só alteração cadastral** — continua MEI | Portal do Empreendedor → "Já sou MEI" → atualização/alteração de dados cadastrais. Gratuito, online, vale após a CCMEI atualizada |
| Novo CNAE **não consta** / é atividade intelectual ou regulamentada (advogado, médico, contador, engenheiro etc.) | **Equivale a desenquadramento** — não cabe como simples alteração | Desenquadramento no Portal do Simples Nacional + migração para ME/EPP |

Pontos práticos:

- O MEI pode ter **1 atividade principal + até 15 secundárias** (16 no total), **todas** dentro da lista permitida.
- Trocar/incluir CNAE **permitido** é livre e a qualquer tempo, sem custo — é só cadastral. Pode **mudar o valor do DAS** (comércio R$ 82,05; serviço R$ 86,05; comércio+serviço R$ 87,05 em 2026) — conferir o boleto do mês seguinte.
- Incluir CNAE **vedado** via alteração cadastral aciona a **comunicação por equivalência**: o sistema desenquadra automaticamente (ver "Comunicação por equivalência").
- Atividade que foi **extinta** da lista do MEI: precisa de desenquadramento no SIMEI; atividade apenas **renomeada/reclassificada** mas ainda permitida: basta atualizar o cadastro.

## Marketplaces, PIX no CPF e cruzamento de dados

Resolve as dúvidas mais frequentes de quem vende online ou recebe em conta pessoal.

### Marketplaces (Shopee, Mercado Livre etc.)

- **O marketplace NÃO retém tributo do MEI.** O MEI paga o **DAS fixo** mensal; não há percentual de imposto retido sobre cada venda na plataforma (isso é diferente da ME no Simples, onde o DAS é variável sobre a receita).
- **Mas o recebimento conta como receita bruta** e alimenta o **cruzamento de dados** da Receita: marketplaces, operadoras de cartão e e-Financeira informam ao Fisco. A Receita confronta esses valores com a DASN-SIMEI.
- A receita é o **valor bruto da venda**, **antes** de descontar comissão da plataforma, frete, taxa de anúncio e embalagem. Essas são **despesas** e não abatem o faturamento. Resultado: dá para estourar o teto mesmo com lucro baixo. Some sempre o **bruto vendido**, não o líquido recebido.

### PIX no CPF (resolvendo a ambiguidade de vez)

A regra é **a origem do dinheiro, não a conta que recebe**:

| Recebimento no CPF (conta pessoal) | Conta no limite do MEI? |
|---|---|
| PIX/transferência/maquininha pela **mesma atividade econômica** do MEI | **Sim** — soma ao faturamento do CNPJ |
| Prestação de serviço/venda relacionada à atividade, recebida no CPF | **Sim** |
| Salário (vínculo empregatício), aposentadoria | **Não** |
| Doações, empréstimos, reembolsos, transferências entre contas próprias | **Não** |

Base: a **Resolução CGSN nº 183/2025** acrescentou ao art. 2º da Res. CGSN 140/2018 dispositivo explicitando que receitas da **mesma atividade econômica** recebidas em inscrições/contas distintas (inclusive no CPF) entram no cálculo do limite. Em nota oficial (nov/2025), a Receita Federal classificou como **fake news** a ideia de que CPF e CNPJ seriam "somados" automaticamente: só conta a renda **vinculada à atividade econômica** do MEI; rendimentos pessoais sem vínculo (salário, doação, empréstimo, simples movimentação na conta) **não** entram — e a Receita afirma que esse critério "sempre foi assim".

> **Status / cautela:** a Res. CGSN 183/2025 está **em vigor** (efeitos a partir de 1º/jan/2026 quanto aos dispositivos sujeitos a noventena; demais artigos, na publicação). Houve forte reação no Congresso e articulação de deputados para **sustar** a norma via projeto de decreto legislativo, mas **não há sustação aprovada** até o momento. Trate a regra de soma CPF+CNPJ como **vigente em sua essência** (a interpretação de que receita da atividade conta sempre valeu); se o cliente perguntar prazos/efeitos exatos ou citar "derrubada no Congresso", **confirme a situação atual da resolução** antes de afirmar. O recado prático não muda: **receber pela atividade no CPF conta para o teto e é rastreável**.

### Não dá para "esconder" faturamento

O cruzamento de PIX, cartões, marketplaces, notas fiscais e e-Financeira ficou sistemático a partir de 2024. Inconsistências típicas detectadas: despesas/compras maiores que a receita declarada, ausência de notas, movimentação acima do padrão de um MEI. A Receita tem promovido exclusões em massa do SIMEI após cruzamento (citam-se milhões de MEIs retirados do regime em 2025) — **confirmar o número** antes de citar valor fechado.

## Fracionar faturamento em vários MEIs/familiares: risco de sonegação

Tentativa comum de "driblar" o teto — e **ilegal**:

- **Cada pessoa (CPF) só pode ter 1 MEI.** O MEI é empresário individual vinculado à figura da pessoa; o próprio sistema do governo **impede** abrir um segundo MEI no mesmo CPF. Quem é MEI também **não pode** ser titular/sócio/administrador de outra empresa.
- Abrir **MEIs em nome de familiares** (cônjuge, filhos, pais) para dividir o faturamento de um mesmo negócio e não estourar os R$ 81.000 é **fragmentação artificial de receita** — a Receita trata como **simulação/sonegação fiscal**.
- O caminho legal para diversificar atividades é **incluir CNAEs secundários no mesmo MEI** (até 16 atividades). Para escalar acima do teto, o caminho é **migrar para ME/EPP**, não pulverizar em CNPJs.
- **Consequências possíveis:** desenquadramento **retroativo**, multa de ofício (pode chegar a 75% do tributo, dobrável em fraude), exclusão do Simples e, em caso de dolo, **crime contra a ordem tributária** (Lei 8.137/90, 2 a 5 anos de reclusão + multa) e até falsidade ideológica. Não minimize: oriente sempre a regularização e a migração legal.

## Comunicação no SIMEI: onde, prazos e multa

**Onde:** Portal do Simples Nacional ou e-CAC → menu SIMEI → Serviços → Desenquadramento → "Comunicação de Desenquadramento do SIMEI". Acesso por código de acesso ou conta gov.br. Seleciona-se o **motivo** e a **data do fato motivador**. Atendimento imediato e gratuito.

**Prazos:**

| Tipo | Prazo de comunicação | Efeito |
|---|---|---|
| Por opção (voluntário), feito **em janeiro** | até o último dia útil de janeiro | desde 1º/jan do **mesmo ano** |
| Por opção, feito de **fevereiro a dezembro** | a qualquer tempo | a partir de 1º/jan do **ano seguinte** |
| Obrigatório — excesso até 20% | último dia útil do mês seguinte ao do excesso | 1º/jan do ano seguinte |
| Obrigatório — excesso > 20% | último dia útil do mês seguinte ao do excesso de 20% | retroativo a 1º/jan do ano |
| Obrigatório — outras vedações (2º empregado, filial, sócio, atividade vedada) | último dia útil do mês seguinte ao evento | a partir do mês subsequente ao evento |

**Multa por falta/atraso da comunicação obrigatória:** R$ 50,00, **insuscetível de redução** (LC 123/2006, art. 36-A). A redução de 50% que existe no SIMEI é de outra penalidade (atraso na entrega da DASN-SIMEI) — não se aplica aqui.

**Não existe desenquadramento "preventivo":** o MEI só comunica depois de efetivamente incorrer na vedação, não por previsão de que vai incorrer. (Para se antecipar, ver "Planejamento preventivo" e o desenquadramento **por opção**.)

## Desenquadramento de ofício

A Receita Federal desenquadra automaticamente quando identifica descumprimento (ex.: excesso de faturamento detectado por cruzamento de dados, atividade vedada, débitos). É **retroativo** ao início do ano-calendário nos casos cabíveis e pode vir com **multa**. Comunicar voluntariamente **dentro do prazo** evita penalidades adicionais. O desenquadramento de ofício por excesso de faturamento e por pendências é tema de fiscalização ativa, com lotes anuais de centenas de milhares de MEIs — se for citar quantitativo específico, **confirmar o número** no portal/notícias da Receita.

## Como voltar a ser MEI

Quem foi desenquadrado/excluído pode pedir o **reenquadramento**, mas:

- A solicitação só pode ser feita **em janeiro** de cada ano-calendário (em geral até o último dia útil de janeiro). Perdeu o prazo → só no próximo ano.
- Pré-requisito: **regularizar todas as pendências** (DAS em atraso, declarações faltantes, débitos em dívida ativa, parcelamentos irregulares, dados cadastrais) no e-CAC.
- Duas etapas no Portal do Simples Nacional: (1) **Opção pelo Simples Nacional**; (2) **Solicitação de opção pelo SIMEI** (enquadramento). Acompanhar pelo serviço "Acompanhamento da Formalização da Opção".
- Deferido, o reenquadramento tem efeito retroativo a 1º/jan do ano.
- **Impeditivo:** se o motivo foi excesso de faturamento **acima de 20%** (> R$ 97.200, Cenário B), **não dá** para voltar a ser MEI no mesmo ano — segue como ME. No Cenário A (excesso de até 20%), pode reenquadrar em janeiro do ano seguinte **se o novo ano-calendário voltar a respeitar o teto**.

## Erros comuns para corrigir no atendimento

- Achar que "ultrapassar o limite" só gera DAS complementar. Só no Cenário A (até 20%); acima de 20% é retroativo e pesado.
- Confundir desenquadramento com baixa: o CNPJ permanece ativo.
- Não contar PIX/transferências/marketplaces como faturamento, ou contar o **líquido** recebido no marketplace em vez do **bruto** vendido.
- Achar que faturamento pontual/não recorrente "não conta": conta — o que vale é o total do ano-calendário.
- Achar que o marketplace "já desconta o imposto": não desconta nada do MEI; o DAS é fixo, mas a venda alimenta o cruzamento.
- Abrir MEI no nome de familiar para dividir receita: é sonegação, com risco de multa pesada e crime.
- Tentar trocar para CNAE vedado como simples alteração cadastral: isso desenquadra automaticamente.
- Não segregar receita de produto (Anexo I) x serviço (Anexo III/V) ao virar ME.
- Não saber que a ME exige contador obrigatório e reserva de caixa para impostos variáveis.
- Tentar reenquadrar fora de janeiro ou sem quitar pendências.

## Quando encaminhar / acionar tool

- **Cálculo de DAS complementar, recálculo retroativo do Simples, escolha de anexo/Fator R, segregação de atividade mista, comparação de carga MEI x ME:** recomendar **contador** — dependem de CNAE, ISS/ICMS municipal/estadual, faturamento e ano de abertura. O agente não deve estimar valor de tributo retroativo definitivo nem honorário fechado.
- **Valores voláteis (DAS do ano, salário mínimo, alíquotas, honorário de contador):** confirmar no **PGMEI / Portal do Simples Nacional** ou em orçamento real antes de afirmar número fechado; mudam por ano, por escritório e por município.
- **Situação da Resolução CGSN 183/2025 (soma CPF+CNPJ):** se o cliente pedir datas/efeitos exatos ou citar "derrubada no Congresso", confirmar se a norma segue em vigor (houve articulação de sustação, sem aprovação até o momento) antes de afirmar prazos.
- **Consulta de situação cadastral, débitos e pendências:** orientar a consulta no e-CAC / "Consulta Optantes" do Portal do Simples Nacional.
- **Alteração de CNAE:** se for atividade permitida, orientar o Portal do Empreendedor (cadastral); se vedada, é caso de desenquadramento/migração.
- Se o cliente já recebeu **cobrança/desenquadramento de ofício**, tratar como urgente: há prazos curtos e multas envolvidas.

## Números-chave de 2026

- Teto MEI: R$ 81.000/ano (R$ 6.750/mês proporcional). MEI Caminhoneiro: R$ 251.600/ano (R$ 20.966,67/mês proporcional) — LC 188/2021.
- 20% acima do teto: R$ 97.200 (ponto de corte da retroatividade). Ritmo do limite dos 20%: ~R$ 8.100/mês.
- Salário mínimo 2026: R$ 1.621,00 (Decreto nº 12.797/2025, vigência 1º/jan/2026); base do INSS do DAS = 5% = R$ 81,05.
- DAS-MEI 2026: Comércio/Indústria R$ 82,05; Serviços R$ 86,05; Comércio + Serviços R$ 87,05 (R$ 81,05 de INSS + R$ 1,00 de ICMS e/ou R$ 5,00 de ISS, conforme atividade). Vencimento dia 20; valores valem a partir da competência jan/2026 (1º pagamento em 20/fev/2026). MEI Caminhoneiro tem INSS de 12% do salário mínimo (DAS mais alto).
- Multa por não comunicar desenquadramento obrigatório: R$ 50,00 (insuscetível de redução).
- Porte ME: até R$ 360.000/ano; EPP: R$ 360.000 a R$ 4.800.000/ano.
- Honorário contábil mensal de ME (ordem de grandeza, NÃO tabelado): ~R$ 137 a R$ 350/mês (contabilidade online básica) a ~R$ 400–R$ 1.200+/mês (escritório tradicional / casos complexos). DAS variável da ME: faixa típica ~4% a ~12% do faturamento conforme anexo e RBT12.
- Cada CPF: no máximo **1 MEI**.

## Fontes

- Portal gov.br — Comunicar desenquadramento do SIMEI: https://www.gov.br/pt-br/servicos/comunicar-desenquadramento-do-simei
- Receita Federal — Manual do Desenquadramento do SIMEI: https://www8.receita.fazenda.gov.br/SimplesNacional/Arquivos/manual/MANUAL_DESENQUADRAMENTO_SIMEI.pdf
- Receita Federal — Manual DASN-SIMEI: https://www8.receita.fazenda.gov.br/SimplesNacional/Arquivos/manual/Manual_DASN-SIMEI.pdf
- Receita Federal — Perguntas e Respostas MEI e SIMEI: https://www8.receita.fazenda.gov.br/simplesnacional/arquivos/manual/perguntaomei.pdf
- Receita Federal — Perguntas e Respostas do Simples Nacional (segregação de receitas / anexos): https://www8.receita.fazenda.gov.br/simplesnacional/arquivos/manual/perguntaosn.pdf
- Receita Federal — DAS-MEI: atualização de valores devidos em 2026: https://www8.receita.fazenda.gov.br/simplesnacional/Noticias/NoticiaCompleta.aspx?id=c3b2044c-ff97-432a-b33c-ecf2a3df6dc3
- Receita Federal — Simples Nacional: entenda as regras da Resolução CGSN nº 183/2025 e se proteja contra fake news (nov/2025): https://www.gov.br/receitafederal/pt-br/assuntos/noticias/2025/novembro/simples-nacional-entenda-as-regras-da-resolucao-cgsn-no-183-2025-e-se-proteja-contra-fake-news
- Planalto — Decreto nº 12.797/2025 (salário mínimo 2026 = R$ 1.621,00): https://www.planalto.gov.br/ccivil_03/_ato2023-2026/2025/decreto/d12797.htm
- gov.br/Secom — MEIs excluídos do Simples Nacional têm até 30 de janeiro para regularizar e voltar (jan/2026): https://www.gov.br/secom/pt-br/acompanhe-a-secom/noticias/2026/01/meis-excluidos-do-simples-nacional-tem-ate-30-de-janeiro-para-regularizar-pendencias-e-voltar-ao-regime-simplificado
- Receita Federal — MEIs excluídos do Simples têm até 31 de janeiro (notícia): https://www8.receita.fazenda.gov.br/simplesnacional/noticias/NoticiaCompleta.aspx?id=3e0cb831-24b7-4cab-b60c-d8e09189e4dd
- Portal do Empreendedor / MEI — Alteração de dados cadastrais e atividades (CNAE): https://mei.receita.economia.gov.br/alteracao
- Sebrae — Mudanças para o MEI: atividades, declaração anual e contratação: https://sebrae.com.br/sites/PortalSebrae/ufs/go/artigos/mudancas-para-o-mei-atividades-declaracao-anual-e-contratacao,02d860ef67f4d610VgnVCM1000004c00210aRCRD
- Agência Brasil — Contribuição mensal do MEI sobe para R$ 81,05 em 2026: https://agenciabrasil.ebc.com.br/economia/noticia/2026-01/recolhimento-do-mei-sobe-para-r-8105-em-2026
- Agência Brasil — Salário mínimo de R$ 1.621 começa a ser pago: https://agenciabrasil.ebc.com.br/economia/noticia/2026-02/salario-minimo-de-r-1621-comeca-ser-pago-nesta-segunda
- Contabeis.com.br — Receita em CPF entra no limite de faturamento do MEI (Res. CGSN 183/2025): https://www.contabeis.com.br/noticias/73748/receita-em-cpf-entra-no-limite-de-faturamento-do-mei/
- CRCSP — Quando o MEI vira fraude? Práticas usadas para sonegar que entram na mira da Receita: https://online.crcsp.org.br/portal/noticias/noticia.asp?c=10709
- CRCMG — MEIs: quais práticas mais expulsam os empreendedores do regime: https://crcmg.org.br/noticias/meis-quais-as-praticas-que-mais-expulsam-os-empreendedores-do-regime-veja-a-lista/
- CartaCapital — Desenquadramento do MEI por faturamento acima do limite: https://www.cartacapital.com.br/do-micro-ao-macro/desenquadramento-do-mei-faturamento-limite-receita/
- E-Auditoria — Segregação de receitas no Simples Nacional: https://www.e-auditoria.com.br/blog/segregacao-de-receitas-simples-nacional/
- Contabilizei — Tabela Simples Nacional 2026 (anexos e alíquotas): https://www.contabilizei.com.br/contabilidade-online/tabela-simples-nacional-completa/
- Agilize — Custo de contabilidade para microempresa no Simples Nacional: https://artigos.agilize.com.br/custo-contabilidade-microempresa-simples-nacional/
- Contajá — Contabilidade online para Simples Nacional (custos 2026): https://contaja.com.br/blog/contabilidade-simples-nacional/
