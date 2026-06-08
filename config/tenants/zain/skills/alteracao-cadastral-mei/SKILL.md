---
name: alteracao-cadastral-mei
description: "Alterar dados do MEI no Portal do Empreendedor: endereço, atividades/CNAE, forma de atuação, contato, capital, nome fantasia e nome social; e CPF suspenso/pendente na Receita. Use quando o cliente quiser mudar/atualizar/corrigir um dado do MEI, incluir ou trocar CNAE, mudar de endereço/cidade, usar nome social, ou disser que não consegue alterar ou que o CPF está bloqueado."
---
# Alterações cadastrais do MEI

## Visão geral do serviço

- **Onde:** Portal do Empreendedor — caminho "Já sou MEI" → card **"Atualização Cadastral de MEI"** → **"Solicitar"**. Domínio oficial: `gov.br/mei`.
- **Custo:** **gratuito**, 100% online. Nunca cobre nem encaminhe o cliente para serviços pagos para alteração cadastral.
- **Quem faz:** o próprio titular, com login **conta gov.br nível Prata ou Ouro** (brasileiros). Estrangeiros: nível Bronze é aceito. Em 2026 o código de acesso do Simples Nacional **não** é mais o caminho para este serviço — é login gov.br.
- **Tempo:** geralmente 5 a 10 minutos; a atualização na base da Receita Federal é **imediata** ao concluir.
- **CCMEI:** ao finalizar, o sistema emite **automaticamente** o novo Certificado da Condição de MEI (CCMEI) já atualizado. Oriente o cliente a baixar e guardar a versão nova.

### Limites operacionais do sistema
- Máximo de **2 alterações cadastrais por dia** para o mesmo CNPJ.
- Até **8 eventos (campos) por solicitação**. Se precisar mexer em mais que isso, conclua uma solicitação e abra outra.
- Atividades: **1 CNAE principal + até 15 secundários** (16 no total). O número de CNAEs **não** altera o teto de faturamento.

## O que PODE x NÃO PODE ser alterado no Portal

### Pode alterar (direto no Portal, gratuito)
- Endereço comercial e/ou residencial
- Atividades econômicas: CNAE **principal** e **secundários** (incluir/remover)
- Forma de atuação (ex.: estabelecimento fixo, internet, ambulante, porta a porta, em local fixo fora da loja, máquinas automáticas etc.)
- Telefone(s) e e-mail
- Capital social
- Documento de identidade
- Nome fantasia (nome de fantasia/comercial) — distinto do nome empresarial

### NÃO pode alterar pelo Portal (vêm da base da Receita / CPF)
Esses dados são puxados do CPF/Receita na formalização e **não** são editáveis no Portal do Empreendedor:
- **CNPJ** (número é vitalício, mesmo em mudança de UF).
- **CPF** do titular.
- **Nome empresarial (razão social):** é padronizado pela Receita = nome civil do empresário + os 8 primeiros dígitos do CNPJ. **Não se altera** pelo Portal. Só muda se o **nome civil mudar no CPF** (ver nome social / retificação de nome abaixo).
- Nome do empresário, data de nascimento, nacionalidade, sexo, nome da mãe → **corrigir primeiro no CPF/Receita Federal**; depois o dado reflete no MEI.

**Pegadinha comum:** cliente pede para "trocar a razão social". Explique que o nome empresarial do MEI não é livre — ele acompanha o nome civil do CPF. Se ele quer outro nome para divulgação, o campo é **nome fantasia** (esse sim editável). Mudança real do nome empresarial exige alterar o nome civil no CPF antes.

## Passo a passo (alteração padrão)

1. Acessar `gov.br/mei` → "Já sou MEI".
2. Abrir "Atualização Cadastral de MEI" → "Solicitar".
3. Fazer login com a conta gov.br (Prata/Ouro).
4. Editar os campos desejados (endereço, atividades, contatos, capital social, forma de atuação etc.).
5. Conferir os dados, ler e marcar as **Declarações** e o **Termo de Ciência e Responsabilidade com Efeito de Dispensa de Alvará e Licença de Funcionamento**.
6. Clicar em "Continuar"/"Finalizar".
7. Emitir e baixar o **novo CCMEI**.

### Específico: incluir/trocar CNAE (principal e secundários)
- No campo "Atividades", manter a principal e adicionar/remover as secundárias buscando o código/ocupação. Cada uma adicionada aparece em lista logo abaixo.
- Para **trocar a principal**, defina o novo CNAE como principal — a anterior pode ser mantida como secundária, se desejado, ou removida.
- **Reinformar todos os códigos** na solicitação — tanto os antigos quanto os novos (a alteração substitui o conjunto, não apenas acrescenta).
- A **mesma atividade não pode ser principal e secundária** ao mesmo tempo.
- Só são aceitas ocupações da **lista oficial de ocupações permitidas ao MEI do ano vigente**. Se o CNAE pretendido não estiver na lista de MEI, a inclusão não é possível pelo Portal (atividade vedada ao MEI) — nesse caso, o caminho é migrar para ME.

## Quando a alteração exige Consulta de Viabilidade (Redesim)

A consulta de viabilidade é necessária **somente** quando a alteração envolve dados de natureza locacional/cadastral verificados na consulta prévia:
- **Endereço**
- **Atividade (CNAE principal e/ou secundária)**
- Nome empresarial, natureza jurídica, tipo de unidade, forma de atuação

Alterações que **não** tocam esses campos (ex.: só telefone, e-mail, capital social) **não** exigem viabilidade e seguem direto.

O que a viabilidade verifica:
- Se a(s) atividade(s) pode(m) ser exercida(s) no endereço escolhido (zoneamento municipal — atenção a zonas residenciais).
- Critérios para dispensa/concessão de alvará de funcionamento (a base consultada é a da **Prefeitura** do município).
- Resultado possível: **Deferida/Admitida** (automático na maioria dos municípios) ou **Indeferida/Sujeita a análise específica** (exige consulta prévia manual da Junta/Prefeitura).
- Validade da consulta de viabilidade: tipicamente em torno de **90 dias**, mas **varia por junta comercial/município** — sempre confirmar o prazo vigente no integrador Redesim local antes de prometer um número ao cliente.

### Mudança de município ou de estado (UF)
- O **CNPJ permanece o mesmo**.
- Para mudança de UF: primeiro **atualizar o endereço do CPF na Receita Federal**; depois fazer a alteração no Portal, com a viabilidade aprovada no estado/município de destino (integrador Redesim do destino).
- Faça **consulta prévia (viabilidade)** antes de efetivar a transferência.

## Nome social (Decreto nº 8.727/2016)

Nome social é a designação pela qual pessoas travestis e transexuais se identificam e são socialmente reconhecidas, com a mesma proteção do nome de registro (Decreto nº 8.727/2016, que regula seu uso na administração pública federal; operacionalizado no CPF pela IN RFB nº 1.718/2017).

**Regra-chave:** o nome social só aparece no MEI se já estiver registrado no **CPF da Receita Federal**. O Portal do Empreendedor/gov.br consulta a base do CPF — sem o registro lá, não há como incluí-lo no MEI.

### Fluxo correto
1. **Incluir o nome social no CPF** (pré-requisito):
   - Online pelo **e-CAC / gov.br** (conta Prata ou Ouro): "Meu CPF" → alterar cadastro. Também por processo digital, e-mail ou atendimento presencial na RFB. **Gratuito.**
   - O nome social passa a constar nos comprovantes do CPF **junto** ao nome civil (coexistem; o nome social **não** substitui o nome civil de registro).
2. **No MEI:**
   - Na **formalização:** marcar "Desejo usar o Nome Social no Nome Empresarial".
   - Em **cadastro já existente:** fazer a atualização cadastral no Portal — só funciona se o nome social já constar no CPF.

**Não confundir:** inclusão de nome social no CPF ≠ retificação do nome civil. A retificação definitiva de nome/gênero (Provimento CNJ 73/2018, direto em cartório) é outro caminho e é o que muda efetivamente o **nome empresarial** do MEI.

## CPF suspenso / "bloqueado" / pendente de regularização

"CPF bloqueado" é termo popular; oficialmente a Receita usa três situações distintas:

| Situação | Causa típica | Como resolver |
|---|---|---|
| **Pendente de regularização** | Falta de entrega da Declaração de IRPF obrigatória nos últimos 5 anos | Entregar a(s) declaração(ões) em atraso (Meu Imposto de Renda / e-CAC). Regulariza em poucos dias. |
| **Suspenso** | Dados cadastrais incorretos/incompletos/desatualizados | Atualizar o cadastro do CPF na Receita ("Atualizar CPF" / e-CAC). |
| **Cancelado/nulo** | Casos específicos (óbito, multiplicidade de inscrições, fraude) | Atendimento da Receita Federal. |

**Impacto no MEI:** CPF irregular pode **bloquear a abertura e a alteração** do MEI (e ainda emissão de passaporte, alguns serviços bancários etc.). Regularizar o CPF é **pré-requisito** — sem isso, a alteração/abertura não passa.

**Custos:** a regularização **online é gratuita**. O atendimento presencial em conveniados (Correios, Banco do Brasil, Caixa) costuma ter uma taxa pequena (na faixa de poucos reais) — confirmar com o canal, pois pode variar; o caminho online evita a cobrança.

**Esclarecimentos para o cliente:**
- Irregularidade cadastral **não** é "nome sujo" (negativação) nem dívida.
- Não declarar IR não bloqueia conta bancária automaticamente.

**Encaminhamento:** para CPF suspenso/pendente, oriente a regularizar pelos canais da Receita Federal (`gov.br/receitafederal` → "Meu CPF") ou atendimento da RFB. Não é resolvível pelo Portal do Empreendedor.

## Depois de alterar: o que mais o cliente precisa fazer

A atualização no Portal alcança a base da Receita Federal, **mas não** se integra automaticamente a todos os órgãos. Após alterar endereço ou atividade, oriente o cliente a:
- Levar o **CCMEI atualizado** à **Prefeitura** e atualizar a inscrição municipal (sem isso pode travar emissão de NFS-e e o alvará).
- Se atua com comércio/indústria e emite NF-e, verificar a **inscrição estadual na Sefaz**.
- Conferir o **DAS do mês seguinte**: mudança de atividade (ex.: passar a ter ISS e/ou ICMS) pode alterar o valor mensal.

### Impacto no valor do DAS (2026)
O DAS-MEI varia conforme os tributos da atividade. Em 2026 o salário mínimo é de **R$ 1.621,00** (Decreto nº 12.797/2025); a parcela de INSS do MEI comum = **5%** = **R$ 81,05/mês** (confirmado pela Receita Federal / Simples Nacional).

| Atividade | Composição | DAS mensal 2026 |
|---|---|---|
| Comércio ou Indústria | INSS R$ 81,05 + ICMS R$ 1,00 | **R$ 82,05** |
| Serviços | INSS R$ 81,05 + ISS R$ 5,00 | **R$ 86,05** |
| Comércio e Serviços (mista) | INSS R$ 81,05 + ICMS R$ 1,00 + ISS R$ 5,00 | **R$ 87,05** |
| MEI Caminhoneiro (transp. autônomo de cargas) | INSS = **12%** = R$ 194,52 (+ ICMS e/ou ISS) | **R$ 195,52 a R$ 200,52** conforme a atividade |

- ICMS (R$ 1,00) e ISS (R$ 5,00) são **fixos desde 2006**, sem reajuste anual. Só o INSS varia, acompanhando o salário mínimo.
- MEI Caminhoneiro, detalhe por tipo de transporte: intermunicipal/interestadual/internacional = R$ 195,52 (INSS + ICMS); municipal = R$ 199,52 (INSS + ISS); produtos perigosos / mudanças (carreto) = R$ 200,52 (INSS + ICMS + ISS).
- Teto de faturamento MEI 2026: **R$ 81.000/ano** (inalterado desde 2018; ≈ R$ 6.750/mês de referência, sem teto mensal rígido). MEI Caminhoneiro: **R$ 251.600/ano** (LC nº 188/2021). Há propostas de aumento em tramitação (PLP 60/2025 — R$ 140 mil; PLP 67/2025 — R$ 150 mil) **não sancionadas** — não afirmar valores novos.
- Para conferir o valor exato e gerar guias, use o **PGMEI** (Programa Gerador do DAS-MEI).

## Erros comuns e pegadinhas
- "Quero mudar a razão social" → não dá pelo Portal; é nome civil + CNPJ. Sugerir **nome fantasia** ou explicar alteração no CPF.
- Tentar incluir CNAE não permitido ao MEI → bloqueia; conferir a lista de ocupações permitidas.
- Incluir CNAE novo e **esquecer de manter os antigos** na solicitação → some o que não foi reinformado.
- Mudar de UF sem antes **atualizar o CPF** → alteração trava.
- Achar que gerar o novo CCMEI encerra tudo → falta atualizar Prefeitura/Sefaz.
- Querer alterar com **CPF pendente/suspenso** → regularizar o CPF primeiro.
- Estourar o limite de **2 alterações/dia** ou **8 eventos/solicitação** → orientar a aguardar/fracionar.

## Quando encaminhar, recusar ou consultar tool
- **CPF suspenso/pendente/cancelado:** encaminhar para a Receita Federal (não é resolvível pelo Portal).
- **Correção de nome civil, data de nascimento, sexo, nome da mãe:** primeiro no CPF/RFB.
- **Retificação definitiva de nome/gênero:** cartório (Provimento CNJ 73/2018) — fora do escopo do MEI.
- **Alteração indevida/fraude no cadastro:** orientar boletim de ocorrência, recuperar a conta gov.br e, após retomar o acesso, atualizar os dados e reemitir o CCMEI.
- **Valor exato de DAS / guias:** consultar PGMEI / tool de geração de DAS antes de afirmar números.
- **Viabilidade indeferida ou atividade de risco:** encaminhar à Prefeitura/Junta Comercial do município.

## Fontes

- Portal do Empreendedor / Alteração (Receita): https://mei.receita.economia.gov.br/alteracao
- gov.br — Realizar atualização de dados cadastrais do MEI: https://www.gov.br/pt-br/servicos/solicitar-alteracao-cadastral-do-microempreendedor-individual
- Simples Nacional — Atualização de valores devidos em 2026 (DAS-MEI): https://www8.receita.fazenda.gov.br/simplesnacional/Noticias/NoticiaCompleta.aspx?id=c3b2044c-ff97-432a-b33c-ecf2a3df6dc3
- Planalto — Decreto nº 12.797/2025 (salário mínimo 2026): https://www.planalto.gov.br/ccivil_03/_ato2023-2026/2025/decreto/d12797.htm
- gov.br — Incluir, alterar ou excluir nome social no CPF: https://www.gov.br/pt-br/servicos/incluir-nome-social-no-cpf
- Receita Federal — Disponibiliza serviço de inclusão e exclusão de nome social no CPF: https://www.gov.br/receitafederal/pt-br/assuntos/noticias/2017/julho/receita-federal-disponibiliza-servico-de-inclusao-e-exclusao-de-nome-social-no-cpf
- Sebrae — Transgêneros podem usar nome social no cadastro do MEI: https://sebrae.com.br/sites/PortalSebrae/artigos/transgeneros-podem-usar-nome-social-no-cadastro-do-mei,2806a20543d47810VgnVCM1000001b00320aRCRD
- Receita Federal — Pedido de regularização do CPF: https://servicos.receita.fazenda.gov.br/servicos/cpf/regularizar/default.asp
- gov.br — Atualizar CPF: https://www.gov.br/pt-br/servicos/atualizar-cadastro-de-pessoas-fisicas
- Redesim — Alterações com viabilidade: https://antigo.redesim.gov.br/servicos/constitua-sua-pj/orientacoes/alteracoes-com-viabilidade
- Sebrae — Alteração das atividades econômicas (CNAE): https://sebrae.com.br/sites/PortalSebrae/artigos/alteracao-das-atividades-economicas-na-cnae,1b38b0927bf09410VgnVCM2000003c74010aRCRD
- Planalto — Decreto nº 8.727/2016 (nome social): https://www.planalto.gov.br/ccivil_03/_ato2015-2018/2016/decreto/d8727.htm
