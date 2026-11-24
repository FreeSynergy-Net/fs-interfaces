//! Rule 20 of `module_v1`: the bytes of an image or file field (`blob`).
//!
//! Pattern: Template Method (a fixed list of rules, each one checked against the service of the fixture).

use super::cx::Cx;

async fn read_all(cx: &Cx<'_>, record: &str, field: &str) -> capnp::Result<[bool; 6]> {
    let mut req = cx.src.blob_request();
    req.get().set_record(record);
    req.get().set_field(field);
    let blob = req.send().promise.await?.get()?.get_blob()?;
    let size = blob.size_request().send().promise.await?;
    let (bytes, media) = (
        size.get()?.get_bytes(),
        size.get()?.get_media_type()?.to_string()?,
    );
    let slash = media.split_once('/');
    let read = |offset: u64, length: u32| {
        let mut r = blob.read_request();
        r.get().set_offset(offset);
        r.get().set_length(length);
        async move { Ok::<_, capnp::Error>(r.send().promise.await?.get()?.get_data()?.len()) }
    };
    let (mut total, mut within, mut ended) = (0u64, true, false);
    for _ in 0..100_000 {
        let n = read(total, 3).await?;
        within &= n <= 3;
        if n == 0 {
            ended = true;
            break;
        }
        total += n as u64;
    }
    Ok([
        bytes > 0,
        slash.is_some_and(|(a, b)| !a.is_empty() && !b.is_empty()),
        within && ended && total == bytes,
        read(bytes, 10).await? == 0,
        read(u64::MAX, 10).await? == 0,
        read(0, 0).await? == 0,
    ])
}

/// Rule 20: blob with an image field: `size` > 0 and a media type `type/subtype`; the chunks (each <= `length`) add up to `size`; reads past the
/// end, at the largest offset and with length 0 are empty, no exception.
pub(crate) async fn blob_bytes(cx: &mut Cx<'_>) {
    let Some(s) = cx.fx.blob_sample() else {
        return cx.skip("20 blob bytes (the fixture has no image or file field)");
    };
    let names = [
        "size is more than 0",
        "media type is type/subtype",
        "chunks are at most length and add up to size",
        "a read past the end is empty",
        "a read at offset u64::MAX is empty",
        "a read with length 0 is empty",
    ];
    match read_all(cx, &s.record, &s.field).await {
        Ok(ok) => {
            for (good, name) in ok.into_iter().zip(names) {
                cx.ensure(good, &format!("20 blob {}: {name}", s.field));
            }
        }
        Err(e) => cx.ensure(false, &format!("20 blob {}: the call failed: {e}", s.field)),
    }
}
