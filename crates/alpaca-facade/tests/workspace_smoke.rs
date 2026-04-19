#[test]
fn workspace_exposes_new_top_level_crates() {
    let _ = std::any::type_name::<alpaca_time::TimeResult<()>>();
    let _ = std::any::type_name::<alpaca_option::OptionResult<()>>();
    let _ = std::any::type_name::<alpaca_facade::FacadeResult<()>>();
}
