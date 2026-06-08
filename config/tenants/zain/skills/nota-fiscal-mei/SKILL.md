---
name: nota-fiscal-mei
description: "Nota fiscal do MEI: quando é obrigatória (PJ x pessoa física), NFS-e (serviço) e NF-e (comércio), Inscrição Estadual/Municipal, ICMS x ISS, retenção indevida pelo tomador, DIFAL e NCM. Use quando perguntarem se precisa dar nota, sobre NFS-e/NF-e, inscrição estadual, retenção de imposto na nota, venda para outro estado, NCM ou erro de limite de notas."
---
# Nota fiscal do MEI

## Conceitos-base

- **MEI** recolhe tributos em valor fixo mensal (DAS-MEI), que **já inclui** ICMS e/ou ISS. Emitir nota **não gera imposto adicional** — o tributo sobre vendas/serviços já está embutido no DAS.
- **NF-e** (Nota Fiscal Eletrônica, modelo 55): documenta **venda de produtos/mercadorias** (comércio e indústria). Imposto associado: **ICMS** (estadual).
- **NFS-e** (Nota Fiscal de Serviço Eletrônica): documenta **prestação de serviços**. Imposto associado: **ISS/ISSQN** (municipal).
- **MEI misto** (comércio + serviço) pode precisar emitir os dois tipos, conforme a operação.

## Quando o MEI é OBRIGADO a emitir nota

| Destinatário da venda/serviço | Obrigatório emitir? |
|---|---|
| Pessoa Jurídica (empresa, qualquer porte) | **Sim, sempre** |
| Órgão público / governo / prefeitura | **Sim, sempre** |
| Pessoa Física (consumidor, CPF) | **Não** — salvo se o cliente solicitar (Código de Defesa do Consumidor) |
| Pessoa Física, com envio/entrega do produto (venda por internet, telefone, catálogo) | **Sim** — para acompanhar o transporte da mercadoria |

Pontos-chave:
- Na venda para **PJ**, a nota pode ser emitida pelo MEI **ou** pelo destinatário (nota de entrada/contra-nota). Base: art. 106, §1º, Resolução CGSN nº 140/2018.
- O MEI **não é obrigado a emitir NF-e** mesmo em vendas interestaduais para consumidor pessoa física — só se quiser. A obrigatoriedade surge quando há entrega/transporte de mercadoria ou destinatário PJ.
- **Pegadinha:** ao vender para PF que NÃO pede nota, o MEI ainda assim deve lançar a receita no Relatório Mensal de Receitas Brutas. "Não emitir nota" ≠ "não declarar a receita".

### O MEI também é obrigado a EXIGIR nota nas compras

Toda compra de fornecedor (inclusive de outro MEI) deve vir com nota fiscal. Essas notas de entrada são guardadas junto ao relatório mensal (ver "Guarda de documentos").

## NFS-e nacional (serviços) — o padrão obrigatório

Desde 1º de setembro de 2023 (Resolução CGSIM nº 169/2022), o **MEI prestador de serviços emite a NFS-e pelo Emissor Nacional** (padrão nacional), **não mais pelo portal da prefeitura** nos municípios que já aderiram — ou seja, a obrigatoriedade do padrão nacional para o MEI **já vigora desde set/2023**, antes da reforma tributária. A partir de **1º/01/2026**, o art. 62 da **LC nº 214/2025** passou a obrigar **todos os municípios e o DF** a autorizar a emissão no padrão nacional (no Emissor Nacional ou compartilhando a NFS-e do emissor próprio com o Ambiente de Dados Nacional — ADN), com migração escalonada ao longo do ano. Para **ME/EPP do Simples** (não MEI), a obrigatoriedade do padrão nacional vai além: passa a valer em **1º/09/2026** (Resolução CGSN nº 189/2026).

- **Onde:** Emissor Nacional em `www.gov.br/nfse` → `www.nfse.gov.br/EmissorNacional`.
- **Custo:** gratuito. MEI **não precisa de certificado digital** para a NFS-e (LC 123/2006).
- **Login:** conta **gov.br nível prata ou ouro** (exigência do Emissor Nacional desde set/2023; o nível bronze não acessa). Quem só tem bronze deve elevar o nível em `gov.br/conta` — reconhecimento facial pelo app gov.br, validação por banco parceiro/internet banking ou certificado digital, em poucos minutos.
- **App:** existe o app NFSe Mobile (consulta/emissão pelo celular).

### Passo a passo (Emissor Nacional)

1. Acesse `www.nfse.gov.br/EmissorNacional` e faça login com a conta gov.br.
2. Na primeira vez, configure o **perfil de emissão** (os dados do prestador vêm do CNPJ).
3. Em "Acesso Rápido", clique em **Emitir NFS-e** (ícone verde com "+").
4. Preencha as 4 telas sequenciais: **Pessoas** (prestador já preenchido; informe o tomador), **Serviço** (descrição/código do serviço), **Valores** e **Emitir**.
5. Confira e emita. A nota fica disponível para download/envio.

### Atenção ao status do município

A migração para o padrão nacional é escalonada em 2026:
- **Município já migrado:** emite **exclusivamente** pelo portal nacional (`nfse.gov.br`); o portal antigo da prefeitura pode estar desativado.
- **Município em transição:** os dois sistemas podem operar em paralelo.
- **Município ainda não migrado:** segue no sistema municipal próprio até concluir a adesão.

Sempre orientar o cliente a confirmar o status no `nfse.gov.br` ou na prefeitura antes de emitir.

## NF-e (comércio/indústria) — venda de produtos

- Emitida pela **SEFAZ do estado** (Secretaria da Fazenda) ou via sistema emissor integrado/gratuito do estado (o Sebrae também oferece emissor gratuito em alguns estados).
- **Requer Inscrição Estadual ativa** e, na prática, **certificado digital** (e-CNPJ) na maioria dos estados.
- Alguns estados oferecem emissor gratuito; em outros é preciso software emissor. Verificar na SEFAZ do estado.

### CRT-4 — código de regime tributário obrigatório na NF-e/NFC-e do MEI

Desde a **Nota Técnica 2024.001** (Ajuste SINIEF 11/19, que alterou o Convênio S/Nº 70), o MEI deixou de usar o CRT 1 (Simples Nacional) e passou a informar o **CRT = 4 (Simples Nacional — MEI)** no campo "Código de Regime Tributário" da **NF-e e da NFC-e**. A obrigatoriedade nacional entrou em vigor em **1º/04/2025** (após adiamento da data original de set/2024).

- Vale **apenas para NF-e/NFC-e (produtos)**. A **NFS-e (serviço) não tem campo CRT** — não muda nada para quem só presta serviço.
- É **só identificação**, não altera a carga tributária. O comprador do MEI **não toma crédito de ICMS**.
- **Erro de "rejeição" na SEFAZ:** emissor desatualizado mandando CRT = 1 gera rejeição (ex.: rejeição 481 em alguns estados, por divergência com o cadastro). Solução: atualizar o emissor e selecionar o código **4**. Há também tabela de CFOP própria para o MEI a observar.

## Inscrição Estadual (IE) e Inscrição Municipal

### Inscrição Estadual (IE) — ICMS

| Atividade do MEI | Precisa de IE? |
|---|---|
| Comércio (loja física, e-commerce, redes sociais, ambulante) | **Sim** |
| Indústria | **Sim** |
| Transporte intermunicipal ou interestadual | **Sim** |
| Serviços puros (consultoria, beleza, manutenção, transporte municipal) | **Não** |

- A IE identifica o MEI como **contribuinte do ICMS** perante a SEFAZ.
- **Mudança 2026:** vários estados endureceram a exigência. O MEI com atividade sujeita ao ICMS **precisa de IE ativa** para emitir NF-e e comprar de fornecedores (inclusive de outros estados). Exemplo: no **Espírito Santo**, IE e emissão de documentos fiscais eletrônicos passaram a ser obrigatórias para MEI sujeito ao ICMS **a partir de 1º/04/2026** (Decreto ES nº 6.335-R/2026), com solicitação pelo Portal Simplifica-ES.
- Em muitos estados a IE é **gerada automaticamente** ao formalizar com CNAE de comércio/indústria; em outros precisa solicitar na SEFAZ/Redesim. **Não tem custo.**
- **Pegadinha:** ter IE **não significa pagar ICMS a mais** — o ICMS já está no DAS-MEI (R$ 1,00/mês fixo). A IE é apenas o cadastro.
- Se o cliente **alterar o CNAE** para incluir comércio/indústria, deve providenciar a IE imediatamente.
- **Consulta gratuita** da IE: portal **SINTEGRA** (`sintegra.gov.br`).
- Sem IE, o MEI de comércio **fica impedido de emitir NF-e** e enfrenta bloqueio de compras de fornecedores.

### Inscrição Municipal — ISS

- Necessária para o prestador de serviço recolher ISS no município. Em geral o cadastro municipal é exigido para liberar a emissão de NFS-e. Verificar na prefeitura/portal nacional.

## ICMS x ISS — resumo

| Imposto | Esfera | Incide sobre | No DAS-MEI 2026 (valor fixo) |
|---|---|---|---|
| **ICMS** | Estadual | Circulação de mercadorias (comércio/indústria/transporte interestadual) | R$ 1,00/mês |
| **ISS/ISSQN** | Municipal | Prestação de serviços | R$ 5,00/mês |

Ambos já estão embutidos no DAS — o MEI **não recolhe ICMS/ISS separadamente** pela operação.

## Retenção de tributos na nota do MEI — em regra é INDEVIDA

Como o MEI recolhe tudo em **valor fixo** no DAS (INSS, ICMS e/ou ISS), o tomador **não deve descontar tributo algum** na nota. Reter implicaria cobrar duas vezes o que já está no DAS. Resumo do que o tomador NÃO deve reter de um MEI:

| Tributo | Retém de MEI? | Base / motivo |
|---|---|---|
| **IRRF** (Imposto de Renda na fonte) | **Não** | Dispensa de retenção sobre pagamentos a optantes do Simples Nacional — art. 1º da IN SRF nº 765/2007. |
| **INSS 11%** (cessão de mão de obra/empreitada) | **Não** | A retenção do art. 31 da Lei 8.212/91 (reproduzida no art. 110 da IN RFB nº 2.110/2022) **não se aplica ao MEI**: ele é tratado como empresa equiparada e recolhe a previdência no próprio DAS (LC 123/2006, arts. 18-A a 18-C). A IN RFB nº 2.289/2025 consolidou as hipóteses de dispensa da retenção previdenciária; jurisprudência do STJ no mesmo sentido. |
| **ISS** na fonte | **Não** | Quando o prestador recolhe ISS por valor fixo (caso do MEI), **não cabe retenção** — art. 21, §4º, IV, da LC 123/2006. |
| **PIS/COFINS/CSLL (CSRF)** | **Não** | Optante do Simples não sofre retenção dessas contribuições (IN SRF nº 765/2007). |

Pontos práticos:
- **Construção civil (INSS):** uma empresa do Simples no **Anexo IV** pode sofrer retenção de 11% — mas isso é regra de ME/EPP. O **MEI não está no Anexo IV** e não sofre essa retenção; a contratação de MEI dispensa a retenção previdenciária na nota. (A obrigação patronal de 20% sobre serviços de obra/reparo, quando existir, é tema do tomador, não retenção na nota do MEI.)
- **Como o cliente reage se o tomador reter mesmo assim (retenção indevida):**
  1. Comunicar o tomador por escrito, apontando que o prestador é **MEI** e citando a base (IRRF: IN SRF 765/2007; INSS: art. 31 da Lei 8.212/91 não se aplica ao Simples; ISS: art. 21, §4º, IV, LC 123/2006). Anexar comprovante de opção pelo Simei.
  2. **IRRF e INSS (federais):** se já foi retido, o pedido de restituição é feito pelo **quem reteve (tomador)** ou pelo prestador via processo administrativo na **Receita Federal** (IN RFB nº 2.055/2021), respeitado o prazo de **5 anos**.
  3. **ISS:** a restituição é sempre **junto à prefeitura** que recebeu o valor (ente competente), nunca na Receita Federal, também no prazo de **5 anos**.
- **Causa comum do erro:** o departamento financeiro do tomador trata o MEI como prestador comum e aplica a tabela de retenção padrão. Em geral basta o MEI sinalizar o regime no momento da contratação/emissão.

> Recuperação de valores retidos, classificação de anexo e casos de construção civil/cessão de mão de obra são tributariamente sensíveis: orientar a regra geral, mas encaminhar a contador para o pedido de restituição/compensação concreto. Ver também a skill irmã sobre **DAS-MEI / tributos do MEI** para a composição do imposto fixo.

## Operações interestaduais — DIFAL na venda x antecipação/DIFAL na compra

Há **dois "DIFAL" diferentes** e o cliente costuma confundi-los. A regra muda conforme seja **venda** (saída) ou **compra** (entrada):

| Operação | MEI recolhe? | Base |
|---|---|---|
| **VENDA** interestadual a **consumidor final não contribuinte** (DIFAL da EC 87/2015) | **Não** — dispensado | STF, **ADI 5469** + **RE 1287019 (Tema 1093)**, jul. 24/02/2021, que declararam inconstitucional a cláusula nona do Convênio ICMS 93/2015. Para o Simples/MEI a modulação **retroage a 12/02/2016** (liminar da ADI 5464). Na saída o MEI só recolhe o que já está no DAS. |
| **COMPRA** interestadual de mercadoria para **revenda**, ou bem para **uso/consumo/ativo** (DIFAL/antecipação da LC 123/2006) | **Sim, em regra** — varia por UF | Art. 13, §1º, XIII, "h" e §5º, e art. 18-A, §3º, VI, da LC 123/2006. **Não** foi afetado pela ADI 5469. Recolhido **fora do DAS**, por **guia estadual própria** (ex.: GARE/GNRE). |

Pontos-chave:
- A **dispensa na venda** (DIFAL-EC 87) é ponto pacificado. O MEI que vende pela internet para consumidor PF de outro estado **não recolhe DIFAL de saída**.
- A **antecipação/DIFAL na compra** existe e **depende de cada estado**: regras, base de cálculo e até a incidência variam por UF (ex.: MG cobra na entrada para revenda; PR pode cobrar só quando a mercadoria entra a 4%; alguns estados isentam mercadorias com substituição tributária — ST). **Sempre verificar a legislação da UF de destino.**
- O **MEI/Simples não usa MVA** (margem de valor agregado) nesse cálculo. Fórmula geral da antecipação/DIFAL de entrada: **(alíquota interna do estado de destino − alíquota interestadual) × base**. Alíquotas interestaduais de referência: **7%** (S/SE → N/NE/CO/ES), **12%** (demais sentidos) e **4%** (importados com conteúdo de importação > 40%).
- Antes de recolher: checar se a mercadoria está em **substituição tributária** (ST) — se já houve ST, em geral não há antecipação. Isso se verifica pela **NCM** e descrição do produto (ver seção NCM) e por convênio/protocolo entre os estados.

> Cálculo e recolhimento da antecipação/DIFAL de compra variam por UF e por produto: orientar a existência da obrigação e o "varia por estado", mas encaminhar a contador/SEFAZ do estado para o valor e a guia corretos. Não chutar alíquotas estaduais.

## NCM — classificação do produto

- **NCM (Nomenclatura Comum do Mercosul):** código de **8 dígitos** que classifica a mercadoria de forma padronizada (baseado no Sistema Harmonizado; os 6 primeiros dígitos = SH internacional, os 2 últimos = especificação do Mercosul). Define enquadramento fiscal, alíquotas e tratamento da mercadoria.
- **Quando o MEI precisa de NCM:** ao emitir **NF-e/NFC-e de produto** (campo obrigatório) e ao **cadastrar produtos em marketplace** (Mercado Livre, Shopee, etc.) — NCM inválido/inexistente é causa frequente de **rejeição da nota** e de o pedido não ser liberado. **Serviço (NFS-e) não usa NCM** (usa código de serviço).
- **Onde consultar:** tabela oficial da **Receita Federal** e **Portal Único Siscomex**; vários emissores e SEFAZ têm busca de NCM. **Na revenda**, o caminho mais seguro é **copiar o NCM da nota de compra do fornecedor** (conferindo se condiz com o produto).
- **Limite do agente:** o agente pode **explicar o que é NCM, onde consultar e por que o código correto importa**, mas **não deve classificar o produto** nem "chutar" o NCM — classificação fiscal incorreta gera autuação/rejeição. Quando o cliente não souber o código, orientar consulta à tabela oficial, à nota do fornecedor ou a contador/Sala do Empreendedor (Sebrae). Não inventar códigos NCM em respostas.

## Quantidade de notas por mês — NÃO há limite

- **Não existe limite de quantidade de notas por mês**, nem valor máximo por nota. O único limite é o **faturamento bruto anual** (R$ 81.000 para MEI geral em 2026; ver "Valores 2026"). Pode-se emitir muitas notas num mês e nenhuma no outro.
- O **valor de referência mensal** (≈ R$ 6.750) é só uma **média anual / 12**, não um teto mensal.
- **Quando o cliente diz que "deu erro de limite" no emissor**, a causa em geral **não é cota de notas** (que não existe), mas sim:
  - **DAS em atraso** (regularizar para liberar emissão);
  - **DASN-SIMEI não entregue** (omissão por 2 anos deixa o CNPJ inapto e bloqueia emissão);
  - **emissor desatualizado** (ex.: falta de **CRT-4**/CFOP novo — ver acima), gerando rejeição;
  - **falta/inatividade de Inscrição Estadual** para NF-e de produto (ver "IE").
- Para o limite anual, **conta toda a receita** (PIX, cartão, dinheiro, com ou sem nota); só **não conta nota cancelada** corretamente. A Receita cruza dados de notas, maquininhas, e-Financeira e marketplaces.

## Desambiguar NF-e x NFS-e ao orientar um cliente PJ que pede nota

Quando um **tomador PJ** pede "a nota" ao MEI, identificar primeiro **o que foi vendido**, pois muda o documento, o portal e a inscrição:

| Se o MEI… | Documento | Onde emite | Precisa de | Imposto (já no DAS) |
|---|---|---|---|---|
| **vendeu produto/mercadoria** | **NF-e** (modelo 55) | **SEFAZ do estado** / emissor estadual | **IE ativa** + (em geral) certificado digital + **CRT-4** | ICMS (R$ 1/mês) |
| **prestou serviço** | **NFS-e** | **Emissor Nacional** (`nfse.gov.br`) ou prefeitura, conforme migração | **Inscrição Municipal**; conta gov.br prata/ouro | ISS (R$ 5/mês) |
| **fez os dois (misto)** | NF-e **e** NFS-e | cada um no seu portal | ambos os cadastros | ICMS + ISS |

- Não orientar o cliente de serviço a "ir à SEFAZ tirar NF-e" — serviço é NFS-e no Emissor Nacional. E não orientar o cliente de produto a usar o `nfse.gov.br`.
- Se o tomador PJ exige um "modelo" específico (ex.: comprador de marketplace pedindo NF-e de produto), confirmar com o cliente o tipo da operação antes de instruir o passo a passo.

## Valores 2026 (atrelados ao salário mínimo)

Salário mínimo 2026: **R$ 1.621,00** (Decreto nº 12.797/2025, vigente desde 1º/01/2026; reajuste de 6,79%). A parcela de INSS do DAS é **5% do salário mínimo = R$ 81,05/mês**. ICMS e ISS são valores fixos e não reajustam. Valores confirmados pelo Portal do Simples Nacional (Receita Federal) para os períodos de apuração de 2026.

| Atividade | Composição do DAS-MEI 2026 | Total mensal |
|---|---|---|
| Comércio ou Indústria | R$ 81,05 (INSS) + R$ 1,00 (ICMS) | **R$ 82,05** |
| Prestação de Serviços | R$ 81,05 (INSS) + R$ 5,00 (ISS) | **R$ 86,05** |
| Comércio e Serviços (misto) | R$ 81,05 + R$ 1,00 + R$ 5,00 | **R$ 87,05** |

- **MEI Caminhoneiro:** INSS = 12% do mínimo = **R$ 194,52/mês** (LC 188/2021). Totais: transporte interestadual de cargas R$ 195,52; municipal R$ 199,52; produtos perigosos/mudanças R$ 200,52.
- **Vencimento do DAS:** dia **20** de cada mês, mesmo sem faturamento. O primeiro DAS com os valores de 2026 (competência jan/2026) venceu em 20/02/2026.
- **Limite de faturamento 2026:** **R$ 81.000/ano** (≈ R$ 6.750/mês) para MEI geral; **R$ 251.600/ano** para MEI Caminhoneiro. Proporcional aos meses ativos se abriu no meio do ano (R$ 6.750 × meses ativos).
- **Tolerância de excesso:** faturar até 20% acima do teto (até R$ 97.200) desenquadra no ano seguinte; ultrapassar 20% desenquadra retroativamente a janeiro.

> Para gerar a guia e confirmar o valor exato do mês, sempre usar o PGMEI/App MEI — não chutar valores nem assumir variações de "transição".

## Faturamento, relatório e declaração (ligados à nota)

- **Relatório Mensal de Receitas Brutas:** preencher **todo mês** (até o dia 20), separando receita de comércio/indústria e de serviços. Não se entrega a ninguém, mas deve ser mantido e ter anexadas as notas de compra e venda. A soma dos 12 relatórios alimenta a declaração anual. Modelo no Portal do Empreendedor.
- **DASN-SIMEI (declaração anual):** obrigatória até **31/05** de cada ano, referente ao ano anterior, mesmo com faturamento zero. Atraso gera MAED: **2% ao mês** sobre os tributos declarados, teto de **20%**, com **mínimo de R$ 50** (redução de 50% para entrega espontânea antes de qualquer ação fiscal). Omissão prolongada deixa o CNPJ inapto, bloqueia emissão de notas e suspende benefícios do INSS; até a baixa de ofício após 3 anos sem declarar e sem pagar.

## Guarda de documentos

- Guardar **todas as notas fiscais emitidas (venda/serviço) e de compra (entrada)** pelo prazo de **5 anos** a contar da emissão.
- Anexar essas notas ao **Relatório Mensal de Receitas Brutas** correspondente.
- Manter os relatórios mensais arquivados por **5 anos** para fins de fiscalização.
- Guardar também comprovantes de eventuais **guias de antecipação/DIFAL de compra** (recolhidas fora do DAS) e de **retenções sofridas indevidamente**, que servem de prova no pedido de restituição.

## Erros comuns / pegadinhas

- Achar que emitir nota gera imposto extra — **não gera**; o tributo já está no DAS fixo.
- MEI de comércio tentar emitir NF-e **sem Inscrição Estadual** — bloqueado.
- Prestador de serviço ainda emitindo pelo **portal antigo da prefeitura** quando o município já migrou para o padrão nacional.
- Não conseguir logar no Emissor Nacional por **conta gov.br abaixo do nível prata/ouro**.
- Vender para PF sem emitir nota e **esquecer de lançar a receita** no relatório mensal.
- Confundir o prazo: DAS vence dia **20**; DASN-SIMEI vence **31/05**.
- Aceitar **retenção de IRRF/INSS/ISS** na nota como se fosse normal — para MEI é, em regra, **indevida**.
- Confundir **DIFAL de venda** (dispensado) com **antecipação/DIFAL de compra** (devido, varia por UF).
- Achar que existe **cota mensal de notas** — não existe; "erro de limite" costuma ser DAS atrasado, DASN pendente ou emissor sem CRT-4.
- Emitir NF-e (produto) e NFS-e (serviço) no portal errado ou com o cadastro errado (IE x Inscrição Municipal).

## Quando encaminhar / consultar tool

- **Valor exato do DAS do mês ou guia para pagamento:** consultar/gerar no PGMEI (App MEI / Portal do Simples Nacional) — usar a tool de emissão/consulta do DAS quando disponível; não chutar o valor.
- **Status de migração do município para a NFS-e nacional** ou regras estaduais específicas de IE (prazos variam por UF): orientar consulta ao `nfse.gov.br` / SEFAZ do estado.
- **Antecipação/DIFAL de compra interestadual:** confirmar a regra e o valor na **SEFAZ da UF de destino** / com contador — varia por estado e por NCM/ST; não chutar alíquotas nem guias.
- **Classificação NCM de um produto específico:** orientar consulta à tabela oficial (Receita/Siscomex), à nota do fornecedor ou a contador/Sebrae; **não classificar nem inventar o código**.
- **Restituição de tributo retido indevidamente (IRRF/INSS via Receita Federal; ISS via prefeitura):** orientar a regra geral e encaminhar a contador para o pedido administrativo concreto.
- **Desenquadramento por excesso de faturamento, ME/Simples, casos tributários complexos ou risco de relação de emprego disfarçada:** encaminhar a contador/Sebrae; fora do escopo desta skill.
- **Emissão efetiva da nota:** orientar o passo a passo; a emissão é feita pelo próprio MEI nos portais oficiais (gov.br/nfse para serviço; SEFAZ para produto).

## Fontes

- Portal do Empreendedor / gov.br — Nota Fiscal do MEI: https://www.gov.br/empresas-e-negocios/pt-br/empreendedor/servicos-para-mei/nota-fiscal
- Emissor Nacional da NFS-e (gov.br): https://www.gov.br/nfse/pt-br
- Receita Federal / Simples Nacional — Atualização de valores devidos pelo MEI em 2026: https://www8.receita.fazenda.gov.br/simplesnacional/Noticias/NoticiaCompleta.aspx?id=c3b2044c-ff97-432a-b33c-ecf2a3df6dc3
- Decreto nº 12.797/2025 (salário mínimo 2026) — Planalto: https://www.planalto.gov.br/ccivil_03/_ato2023-2026/2025/decreto/d12797.htm
- Agência Brasil — Contribuição do MEI sobe para R$ 81,05 em 2026: https://agenciabrasil.ebc.com.br/economia/noticia/2026-01/recolhimento-do-mei-sobe-para-r-8105-em-2026
- Agência Brasil — Salário mínimo de R$ 1.621 em 2026: https://agenciabrasil.ebc.com.br/economia/noticia/2025-12/novo-salario-minimo-sera-de-r-1621-em-2026
- gov.br — Declaração Anual de Faturamento (DASN-SIMEI) e multa por atraso: https://www.gov.br/empresas-e-negocios/pt-br/empreendedor/servicos-para-mei/declaracao-anual-de-faturamento
- SEFAZ-ES — IE e documentos fiscais obrigatórios para MEI a partir de 1º/04/2026: https://sefaz.es.gov.br/Not%C3%ADcia/inscricao-estadual-e-emissao-de-documentos-fiscais-serao-obrigatorios-para-mei-a-partir-de-1o-de-abril-de-2026
- Sebrae — MEI precisa de inscrição estadual?: https://sebrae.com.br/sites/PortalSebrae/artigos/mei-precisa-de-inscricao-estadual,44687a3a415f5810VgnVCM1000001b00320aRCRD
- Sebrae — Relatório Mensal de Receitas Brutas MEI: https://sebrae.com.br/empreendedores/guiaparaomei/ja-sou-mei/orientacoes-e-ferramentas-mei/relatorio-mensal-de-receitas-brutas-mei
- LC nº 123/2006 (Estatuto da ME/EPP; art. 13, art. 18-A, art. 21) — Planalto: https://www.planalto.gov.br/ccivil_03/leis/lcp/lcp123.htm
- Lei nº 8.212/1991, art. 31 (retenção de 11% — não aplicável ao Simples/MEI) — Planalto: https://www.planalto.gov.br/ccivil_03/leis/l8212cons.htm
- IN SRF nº 765/2007 (dispensa de retenção de IRRF/CSRF sobre pagamentos a optantes do Simples Nacional): http://normas.receita.fazenda.gov.br/sijut2consulta/link.action?idAto=15407
- IN RFB nº 2.110/2022 (normas de arrecadação previdenciária; art. 110 reproduz a retenção do art. 31 da Lei 8.212/91; dispensa para o MEI): http://normas.receita.fazenda.gov.br/sijut2consulta/link.action?idAto=127814
- Receita Federal — IN RFB nº 2.289/2025 (consolida as hipóteses de dispensa da retenção previdenciária de 11%): https://www.gov.br/receitafederal/pt-br/assuntos/noticias/2025/novembro/receita-federal-consolida-hipoteses-de-dispensa-da-retencao-previdenciaria-em-contratos-de-servicos-e-obras
- IN RFB nº 2.055/2021 (restituição, compensação e ressarcimento): http://normas.receita.fazenda.gov.br/sijut2consulta/link.action?idAto=121289
- STF — ADI 5469 e RE 1287019 / Tema 1093 (DIFAL da EC 87/2015 e cláusula nona do Convênio ICMS 93/2015; inconstitucionalidade, com modulação retroativa a 12/02/2016 para o Simples/MEI): https://portal.stf.jus.br/processos/detalhe.asp?incidente=4910011
- LC nº 214/2025, art. 62 (reforma tributária; padrão nacional da NFS-e obrigatório para municípios a partir de 1º/01/2026): https://www.planalto.gov.br/ccivil_03/leis/lcp/lcp214.htm
- Resolução CGSN nº 189/2026 (NFS-e de padrão nacional obrigatória para ME/EPP do Simples a partir de 1º/09/2026): https://www.gov.br/receitafederal/pt-br
- CONFAZ — Convênio ICMS nº 93/2015 (DIFAL EC 87): https://www.confaz.fazenda.gov.br/legislacao/convenios/2015/CV093_15
- Portal Único Siscomex — consulta de NCM / Nomenclatura Comum do Mercosul: https://www.gov.br/siscomex/pt-br
- Contabilizei — CRT-4 para MEI (Nota Técnica 2024.001 / Ajuste SINIEF 11/19, obrigatório desde 1º/04/2025): https://www.contabilizei.com.br/contabilidade-online/crt-4-para-mei/
