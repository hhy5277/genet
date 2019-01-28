use crate::{
    codable::{Codable, CodedData},
    context::Context,
    layer::LayerStack,
    package::IntoBuilder,
    result::Result,
    string::SafeString,
};
use failure::format_err;
use serde_derive::{Deserialize, Serialize};
use std::ptr;

/// Decoding status.
#[derive(Clone, PartialEq, Debug)]
pub enum Status {
    Done,
    Skip,
}

/// Decoder worker trait.
pub trait Worker {
    fn decode(&mut self, stack: &mut LayerStack) -> Result<Status>;
}

pub struct DecoderStack {
    worker: WorkerBox,
    sub_workers: Vec<DecoderStack>,
}

impl DecoderStack {
    pub fn new(worker: WorkerBox, sub_workers: Vec<DecoderStack>) -> DecoderStack {
        Self {
            worker,
            sub_workers,
        }
    }

    pub fn decode(&mut self, layer: &mut LayerStack) -> Result<Status> {
        match self.worker.decode(layer) {
            Ok(Status::Done) => {
                for worker in self.sub_workers.iter_mut() {
                    let _ = worker.decode(layer);
                }
                Ok(Status::Done)
            }
            Ok(Status::Skip) => Ok(Status::Skip),
            Err(err) => Err(err),
        }
    }
}

#[repr(C)]
pub struct WorkerBox {
    decode: extern "C" fn(*mut WorkerBox, *mut LayerStack, *mut SafeString) -> u8,
    drop: extern "C" fn(*mut Box<Worker>),
    worker: *mut Box<Worker>,
}

impl WorkerBox {
    fn new(worker: Box<Worker>) -> WorkerBox {
        Self {
            decode: abi_decode,
            drop: abi_drop,
            worker: Box::into_raw(Box::new(worker)),
        }
    }

    pub fn decode(&mut self, layer: &mut LayerStack) -> Result<Status> {
        let mut err = SafeString::new();
        let result = (self.decode)(self, layer, &mut err);
        match result {
            2 => Ok(Status::Done),
            1 => Ok(Status::Skip),
            _ => Err(format_err!("{}", err)),
        }
    }
}

impl Drop for WorkerBox {
    fn drop(&mut self) {
        (self.drop)(self.worker);
    }
}

extern "C" fn abi_decode(
    worker: *mut WorkerBox,
    layer: *mut LayerStack,
    error: *mut SafeString,
) -> u8 {
    let worker = unsafe { &mut *((*worker).worker) };
    let mut layer = unsafe { &mut *layer };
    match worker.decode(&mut layer) {
        Ok(stat) => match stat {
            Status::Done => 2,
            Status::Skip => 1,
        },
        Err(err) => {
            unsafe {
                ptr::write(error, SafeString::from(&format!("{}", err)));
            }
            0
        }
    }
}

extern "C" fn abi_drop(worker: *mut Box<Worker>) {
    unsafe { Box::from_raw(worker) };
}

/// Decoder trait.
pub trait Decoder: DecoderClone + Send {
    fn new_worker(&self, ctx: &Context) -> Box<Worker>;
}

pub trait DecoderClone {
    fn clone_box(&self) -> Box<Decoder>;
}

impl<T> DecoderClone for T
where
    T: 'static + Decoder + Clone,
{
    fn clone_box(&self) -> Box<Decoder> {
        Box::new(self.clone())
    }
}

impl Clone for Box<Decoder> {
    fn clone(&self) -> Box<Decoder> {
        self.clone_box()
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct DecoderBox {
    new_worker: extern "C" fn(*const DecoderBox, *const Context) -> WorkerBox,
    decoder: *mut Box<Decoder>,
}

unsafe impl Send for DecoderBox {}
unsafe impl Codable for DecoderBox {}

impl DecoderBox {
    pub fn new<T: 'static + Decoder>(diss: T) -> DecoderBox {
        let diss: Box<Decoder> = Box::new(diss);
        Self {
            new_worker: abi_new_worker,
            decoder: Box::into_raw(Box::new(diss)),
        }
    }

    pub fn new_worker(&self, ctx: &Context) -> WorkerBox {
        (self.new_worker)(self, ctx)
    }
}

extern "C" fn abi_new_worker(diss: *const DecoderBox, ctx: *const Context) -> WorkerBox {
    let diss = unsafe { &*(*diss).decoder };
    let ctx = unsafe { &(*ctx) };
    WorkerBox::new(diss.new_worker(ctx))
}

impl<T: 'static + Decoder> IntoBuilder<DecoderData> for T {
    fn into_builder(self) -> DecoderData {
        DecoderData {
            id: String::new(),
            trigger_after: Vec::new(),
            decoder: CodedData::new(DecoderBox::new(self)),
        }
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct DecoderData {
    pub id: String,
    pub trigger_after: Vec<String>,
    pub decoder: CodedData<DecoderBox>,
}

impl DecoderData {
    pub fn id<T: Into<String>>(mut self, id: T) -> Self {
        self.id = id.into();
        self
    }

    pub fn trigger_after<T: Into<String>>(mut self, id: T) -> Self {
        self.trigger_after.push(id.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        attr::AttrClass,
        context::Context,
        decoder::{Decoder, DecoderBox, Status, Worker},
        fixed::Fixed,
        layer::{Layer, LayerClass, LayerStack, LayerStackData},
        result::Result,
        slice::ByteSlice,
        token::Token,
    };

    #[test]
    fn decode() {
        struct TestWorker {}

        impl Worker for TestWorker {
            fn decode(&mut self, stack: &mut LayerStack) -> Result<Status> {
                let attr = vec![Fixed::new(AttrClass::builder(Token::from(1234)).build())];
                let class = Box::new(Fixed::new(LayerClass::builder(attr).build()));
                let layer = Layer::new(&class, &ByteSlice::new());
                stack.add_child(layer);
                Ok(Status::Done)
            }
        }

        #[derive(Clone)]
        struct TestDecoder {}

        impl Decoder for TestDecoder {
            fn new_worker(&self, _ctx: &Context) -> Box<Worker> {
                Box::new(TestWorker {})
            }
        }

        let ctx = Context::default();
        let diss = DecoderBox::new(TestDecoder {});
        let mut worker = diss.new_worker(&ctx);

        let attr = vec![Fixed::new(AttrClass::builder(Token::null()).build())];
        let class = Box::new(Fixed::new(LayerClass::builder(attr).build()));
        let mut layer = Layer::new(&class, &ByteSlice::new());
        let mut data = LayerStackData {
            children: Vec::new(),
        };
        let mut layer = LayerStack::from_mut_ref(&mut data, &mut layer);

        assert_eq!(worker.decode(&mut layer).unwrap(), Status::Done);
    }
}
