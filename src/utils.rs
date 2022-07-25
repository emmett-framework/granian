pub(crate) fn header_contains_value(
    headers: &hyper::HeaderMap,
    header: impl hyper::header::AsHeaderName,
    value: impl AsRef<[u8]>
) -> bool {
    let value = value.as_ref();
    for header in headers.get_all(header) {
        if header.as_bytes().split(|&c| c == b',').any(
            |x| trim(x).eq_ignore_ascii_case(value)
        ) {
            return true;
        }
    }
    false
}

fn trim(data: &[u8]) -> &[u8] {
    trim_end(trim_start(data))
}

#[inline]
fn trim_start(data: &[u8]) -> &[u8] {
    if let Some(start) = data.iter().position(|x| !x.is_ascii_whitespace()) {
        &data[start..]
    } else {
        b""
    }
}

#[inline]
fn trim_end(data: &[u8]) -> &[u8] {
    if let Some(last) = data.iter().rposition(|x| !x.is_ascii_whitespace()) {
        &data[..last + 1]
    } else {
        b""
    }
}
