-- Marca quando um mês do DAS foi detectado em PARCELAMENTO.
--
-- O status "parcelado" só aparece ao tentar EMITIR a guia (o portal mostra
-- um toast; a tabela de consulta não distingue parcelado de devedor). Sem
-- cachear, o `emitir_das` reabriria o browser e tentaria gerar TODA vez que
-- o agente pedisse aquele mês — desperdício (e consome o limite diário).
--
-- Aqui guardamos `parcelado_em`: quando setado (e recente), o `emitir_das`
-- corta o circuito e devolve "parcelado" sem RPA. O `das_refresh` PRESERVA
-- esse campo enquanto o mês segue em aberto, e o ZERA quando o mês vira
-- liquidado/não-optante (parcelamento quitado ou não mais aplicável).
ALTER TABLE zain.das_mensal
    ADD COLUMN parcelado_em TIMESTAMPTZ;
