//! Probe do login gov.br real, usando o código de produção
//! (`rpa::govbr::check_govbr_profile`) com logs por etapa.
//!
//! Uso: `govbr_probe <CPF> <SENHA> [OTP]`

use rpa::govbr;

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        if std::env::var("RUST_LOG").is_err() {
            std::env::set_var("RUST_LOG", "rpa=debug,chromium_driver=info");
        }
    }
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_target(true)
        .init();

    let mut args = std::env::args().skip(1);
    let cpf = args.next().expect("uso: govbr_probe <CPF> <SENHA> [OTP]");
    let password = args.next().expect("uso: govbr_probe <CPF> <SENHA> [OTP]");
    let otp = args.next();

    let t0 = std::time::Instant::now();
    eprintln!("[probe] iniciando login gov.br (sem sessão salva)…");

    let result = govbr::check_govbr_profile(&cpf, &password, otp.as_deref(), None).await;

    eprintln!("[probe] terminou em {:.1}s", t0.elapsed().as_secs_f32());
    match result {
        Ok(outcome) => {
            eprintln!("[probe] SUCESSO fresh_login={}", outcome.fresh_login);
            eprintln!("[probe] perfil: {:?}", outcome.profile);
            eprintln!(
                "[probe] cookies capturados: {}",
                outcome.session.cookies.len()
            );
        }
        Err(e) => {
            eprintln!("[probe] FALHA: {e}");
            eprintln!("[probe] debug: {e:?}");
        }
    }
    Ok(())
}
