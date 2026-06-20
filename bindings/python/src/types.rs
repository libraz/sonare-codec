use super::*;

#[pyclass]
pub(crate) struct StreamDecoder {
    inner: sonare_codec_rs::StreamDecoder,
}

#[pymethods]
impl StreamDecoder {
    #[new]
    fn new() -> Self {
        Self {
            inner: sonare_codec_rs::StreamDecoder::new(),
        }
    }

    fn decode_stream(&mut self, chunk: &[u8]) -> PyResult<Option<(u32, u16, Vec<f32>)>> {
        self.inner
            .decode_stream(chunk)
            .map(|decoded| decoded.map(pcm_tuple))
            .map_err(to_py_value_error)
    }

    fn reset(&mut self) {
        self.inner.reset();
    }

    fn buffered_len(&self) -> usize {
        self.inner.buffered_len()
    }
}
