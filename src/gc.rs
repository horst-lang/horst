use std::any::{Any, type_name};
use std::fmt;
use std::fmt::Debug;
use std::marker::PhantomData;

pub trait GcTrace {
    fn size(&self) -> usize;
    fn trace(&self, gc: &mut Gc);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub struct GcRef<T: GcTrace> {
    index: usize,
    _marker: std::marker::PhantomData<T>,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub struct GcRefRaw {
    index: usize,
}

impl GcRefRaw {
    fn to_gc_ref<T: GcTrace>(&self) -> GcRef<T> {
        return GcRef {
            index: self.index,
            _marker: PhantomData
        }
    }
}

impl<T: GcTrace> Copy for GcRef<T> {}
impl<T: GcTrace> Eq for GcRef<T> {}

impl<T: GcTrace> Clone for GcRef<T> {
    #[inline]
    fn clone(&self) -> GcRef<T> {
        *self
    }
}

impl<T: GcTrace> Debug for GcRef<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let full_name = type_name::<T>();
        full_name.split("::").last().unwrap();
        write!(f, "ref({}:{})", self.index, full_name)
    }
}

impl<T: GcTrace> PartialEq for GcRef<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

struct GcObjectHeader {
    marked: bool,
    size: usize,
    obj: Box<dyn GcTrace>,
}

pub struct Gc {
    bytes_allocated: usize,
    next_gc: usize,
    free_slots: Vec<usize>,
    objects: Vec<Option<GcObjectHeader>>,
}

impl Gc {
    const INITIAL_HEAP_SIZE: usize = 1024 * 1024;

    pub(crate) fn new() -> Gc {
        Gc {
            bytes_allocated: 0,
            next_gc: Self::INITIAL_HEAP_SIZE,
            free_slots: Vec::new(),
            objects: Vec::new(),
        }
    }

    pub fn alloc<T: GcTrace + 'static>(&mut self, obj: T) -> GcRef<T> {
        let size = obj.size() + std::mem::size_of::<GcObjectHeader>();
        self.bytes_allocated += size;
        let entry = GcObjectHeader {
            marked: false,
            size,
            obj: Box::new(obj),
        };
        let index = if let Some(i) = self.free_slots.pop() {
            self.objects[i] = Some(entry);
            i
        } else {
            self.objects.push(Some(entry));
            self.objects.len() - 1
        };

        GcRef {
            index,
            _marker: PhantomData,
        }
    }

    pub fn deref<T: GcTrace + 'static>(&self, r: GcRef<T>) -> &T {
        self.objects[r.index]
            .as_ref()
            .unwrap()
            .obj
            .as_any()
            .downcast_ref::<T>()
            .unwrap_or_else(|| panic!("Reference to wrong type"))
    }

    pub fn deref_mut<T: GcTrace + 'static>(&mut self, r: GcRef<T>) -> &mut T {
        self.objects[r.index]
            .as_mut()
            .unwrap()
            .obj
            .as_any_mut()
            .downcast_mut::<T>()
            .unwrap_or_else(|| panic!("Reference to wrong type"))
    }

    pub fn free<T: GcTrace + 'static>(&mut self, r: GcRef<T>) {
        if let Some(obj) = self.objects[r.index].take() {
            self.bytes_allocated -= obj.size;
            self.free_slots.push(r.index);
        } else {
            panic!("Double free");
        }
    }
}