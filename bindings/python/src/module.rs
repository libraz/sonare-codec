use super::*;

#[pymodule]
pub(crate) fn sonare_codec(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<StreamDecoder>()?;
    decode::add_py_functions(module)?;
    encode_basic::add_py_functions(module)?;
    aac_encode::add_py_functions(module)?;
    aac_tables::add_py_functions(module)?;
    aac_diagnostics::add_py_functions(module)?;
    mp3_diagnostics::add_py_functions(module)?;
    Ok(())
}
