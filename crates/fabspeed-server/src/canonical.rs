//! FAB-C14N-1: typed canonical bytes, independent of JSON decimal formatting.
//! See docs/fabapps/catalogue/v1/README.md. Not an authenticity signature.
use serde_json::Value;
use sha2::{Digest, Sha256};

pub fn bytes(value: &Value) -> Result<Vec<u8>, &'static str> {
    fn write(v: &Value, out: &mut Vec<u8>) -> Result<(), &'static str> {
        match v {
            Value::Null => out.push(b'n'),
            Value::Bool(b) => out.push(if *b { b't' } else { b'f' }),
            Value::Number(n) => {
                if n.as_u64().is_some_and(|i| i > 9_007_199_254_740_991)
                    || n.as_i64().is_some_and(|i| i < -9_007_199_254_740_991)
                {
                    return Err("integer outside interoperable range");
                }
                let f = n
                    .as_f64()
                    .filter(|v| v.is_finite())
                    .ok_or("invalid number")?;
                out.push(b'd');
                out.extend_from_slice(&(if f == 0.0 { 0f64 } else { f }).to_bits().to_be_bytes());
            }
            Value::String(s) => {
                out.push(b's');
                out.extend_from_slice(&(s.len() as u64).to_be_bytes());
                out.extend_from_slice(s.as_bytes());
            }
            Value::Array(a) => {
                out.push(b'a');
                out.extend_from_slice(&(a.len() as u64).to_be_bytes());
                for v in a {
                    write(v, out)?;
                }
            }
            Value::Object(m) => {
                out.push(b'o');
                out.extend_from_slice(&(m.len() as u64).to_be_bytes());
                let mut keys: Vec<_> = m.keys().collect();
                keys.sort();
                for k in keys {
                    write(&Value::String(k.clone()), out)?;
                    write(&m[k], out)?;
                }
            }
        }
        Ok(())
    }
    let mut out = Vec::new();
    write(value, &mut out)?;
    Ok(out)
}
pub fn digest(domain: &str, value: &Value) -> Result<String, &'static str> {
    let mut h = Sha256::new();
    h.update(b"fabapps-c14n-1\0");
    h.update(domain.as_bytes());
    h.update([0]);
    h.update(bytes(value)?);
    Ok(format!("sha256:{:x}", h.finalize()))
}
