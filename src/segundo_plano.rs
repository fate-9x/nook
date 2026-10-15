//! Un hilo aparte para lo que tarda en pedirse a Windows (las apps instaladas, los iconos), para
//! que la interfaz no se quede esperando. Lo que devuelve cada trabajo se recoge después en el
//! hilo de la interfaz, que es donde están ella y el estado (que no se pueden pasar a otro hilo).

use std::any::Any;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::sync::LazyLock;
use std::sync::mpsc::{self, Sender};
use windows::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx};

type Trabajo = Box<dyn FnOnce() + Send>;
type Respuesta = Box<dyn FnOnce(Box<dyn Any + Send>)>;

/// Los trabajos se hacen de uno en uno, en el orden en que se piden.
static HILO: LazyLock<Sender<Trabajo>> = LazyLock::new(|| {
    let (envio, recibo) = mpsc::channel::<Trabajo>();
    let _ = std::thread::Builder::new().name("segundo-plano".into()).spawn(move || {
        // Las funciones del shell (la carpeta de aplicaciones, los iconos) piden COM de un solo hilo.
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        }
        for trabajo in recibo {
            trabajo();
        }
    });
    envio
});

thread_local! {
    /// Lo que hay que hacer con el resultado de cada trabajo en curso (en el hilo de la interfaz).
    static ESPERANDO: RefCell<HashMap<u64, Respuesta>> = RefCell::new(HashMap::new());
    static SIGUIENTE: Cell<u64> = const { Cell::new(0) };
}

/// Hace `trabajo` en el hilo aparte y, cuando termina, `listo` con su resultado en el hilo de la
/// interfaz. Hay que llamarla desde el hilo de la interfaz.
pub fn hacer<T: Send + 'static>(trabajo: impl FnOnce() -> T + Send + 'static, listo: impl FnOnce(T) + 'static) {
    let id = SIGUIENTE.with(|s| s.replace(s.get() + 1));
    let respuesta: Respuesta = Box::new(move |resultado| {
        if let Ok(resultado) = resultado.downcast::<T>() {
            listo(*resultado);
        }
    });
    ESPERANDO.with(|e| e.borrow_mut().insert(id, respuesta));
    let enviado = HILO.send(Box::new(move || {
        let resultado: Box<dyn Any + Send> = Box::new(trabajo());
        let _ = slint::invoke_from_event_loop(move || {
            if let Some(respuesta) = ESPERANDO.with(|e| e.borrow_mut().remove(&id)) {
                respuesta(resultado);
            }
        });
    }));
    if enviado.is_err() {
        ESPERANDO.with(|e| e.borrow_mut().remove(&id));
    }
}
