/*
  ____                 __               __ __                 __
 / __ \__ _____ ____  / /___ ____ _    / //_/__ _______  ___ / /
/ /_/ / // / _ `/ _ \/ __/ // /  ' \  / ,< / -_) __/ _ \/ -_) /
\___\_\_,_/\_,_/_//_/\__/\_,_/_/_/_/ /_/|_|\__/_/ /_//_/\__/_/
  Part of the Quantum OS Kernel

Copyright 2022 Gavin Kellam

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and
associated documentation files (the "Software"), to deal in the Software without restriction,
including without limitation the rights to use, copy, modify, merge, publish, distribute,
sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial
portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT
NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND
NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT
OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.

*/

use core::marker::PhantomData;
use core::mem::size_of;
use crate::error_utils::QuantumError;

#[derive(Debug)]
struct RecursiveComponent<'a, T> {
    ptr: &'a mut [u8],
    ph: PhantomData<T>
}

struct ComponentInformation<'a,  T> {
    data_ptr: *const T,
    total: usize,
    used: usize,
    next_ptr: *const RecursiveComponent<'a, T>
}

// Data stored in beginning of ptr
// (Next Ptr) (Used) [DATA] (END FILLER)
// Overhead is as minimal as possible

impl<'a, T> RecursiveComponent<'a, T> {
    pub fn new(bytes: &'a mut [u8]) -> Result<Self, QuantumError> {
        let total_bytes_size = bytes.len();
        let size_of_overhead = size_of::<(Self, usize)>();
        let data_section_size = total_bytes_size - size_of_overhead;

        if total_bytes_size <= size_of_overhead && data_section_size <= size_of::<T>() {
            return Err(QuantumError::NoSpaceRemaining);
        }

        let raw_new = Self {
            ptr: bytes,
            ph: PhantomData::default()
        };

        Ok(raw_new)
    }

    pub fn get_buffer_info(&mut self) -> ComponentInformation<T> {
        let ptr = self.ptr.as_mut_ptr() as *mut u8;
        let total_size_of_buffer= self.ptr.len();
        let size_of_over_head = size_of::<(Self, usize)>();

        let size_of_data_section = total_size_of_buffer - size_of_over_head;
        let total_fitting_allocations = size_of_data_section / size_of::<T>();

        let next_vector_ptr = ptr as *mut Self;

        let info_ptr = ptr as *mut usize;
        let used_data = unsafe { *(info_ptr.add(2)) };

        let shifted_ptr = unsafe { info_ptr.add(3) };
        let data_ptr = shifted_ptr as *mut T;

        ComponentInformation::<T> {
            data_ptr,
            total: total_fitting_allocations,
            used: used_data as usize,
            next_ptr: next_vector_ptr
        }
    }

    fn modify_used(&mut self, modify: isize) {
        let ptr = self.ptr.as_mut_ptr() as *mut u64;
        let size_ref = unsafe  { &mut *(ptr.add(2)) };
        *size_ref = (*size_ref as isize + modify) as u64;
    }

    pub fn push(&mut self, element: T) -> Result<(), QuantumError> {
        let self_info = self.get_buffer_info();
        let data_ptr = self_info.data_ptr as *mut T;

        // check if new data fits
        if self_info.total <= self_info.used {
            return Err(QuantumError::BufferFull);
        }

        // add the data to the buffer
        unsafe { *(data_ptr.add(self_info.used)) = element };
        self.modify_used(1);

        Ok(())
    }

    pub fn set(&mut self, index: usize, element: T) -> Result<(), QuantumError> {
        let buffer_info = self.get_buffer_info();
        let data_ptr = buffer_info.data_ptr as *mut T;

        // check if the index is within already allocated range, we cant have an element placed
        // in the middle of the vector (e.g [N, N, N, 0, N])
        if index > buffer_info.used {
            return Err(QuantumError::OverflowValue);
        } else if index == buffer_info.used {
            self.modify_used(1);
        }
        

        unsafe { *(data_ptr.add(index)) = element  };

        Ok(())
    }

    pub fn get(&mut self, key: usize) -> Result<&mut T, QuantumError> {
        let self_info = self.get_buffer_info();

        if key >= self_info.used {
            return Err(QuantumError::NoItem);
        }

        let data_ptr = self_info.data_ptr as *mut T;

        let value = unsafe {
            &mut *data_ptr.add(key)
        };

        Ok(value)
    }

    pub fn remove_element(&mut self, key: usize) {
        let self_info = self.get_buffer_info();
        let data_ptr = self_info.data_ptr;

        for i in (key + 1)..self_info.used {
            let prev_index = i - 1;
            let prev_ptr = unsafe { data_ptr.add(prev_index) as *mut T };
            let current_ptr = unsafe { data_ptr.add(i) as *mut T };

            unsafe {
                *prev_ptr = core::ptr::read_unaligned(current_ptr);
            }
        }

        self.modify_used(-1);
    }

    pub fn len(&mut self) -> usize {
        self.get_buffer_info().used
    }

    pub fn total_size(&mut self) -> usize {
        self.get_buffer_info().total
    }

    pub fn is_full(&mut self) -> bool {
        let info = self.get_buffer_info();

        info.used >= info.total
    }

    pub fn recurse_next_component(&mut self, component: Self) -> Result<(), QuantumError> {
        if self.is_parent() {
            return Err(QuantumError::ExistingValue);
        }

        let self_ptr = self.ptr.as_mut_ptr() as *mut Self;
        unsafe { *self_ptr = component  };

        Ok(())
    }

    pub fn is_parent(&mut self) -> bool {
        let next_comp = self.get_buffer_info().next_ptr as *mut Self;
        let address = next_comp as u64;

        if address > 0 {
            let next_info = unsafe { &mut *next_comp };
            return next_info.ptr.as_ptr() as u64 > 0;
        }

        false
    }

    pub fn get_child(&mut self) -> Option<*mut Self> {
        if self.is_parent() {
            let next_comp = self.get_buffer_info().next_ptr as *mut Self;
            let child = next_comp ;

            return Some(child);
        }

        None
    }

    pub fn get_bottom(&mut self) -> *mut Self {
        if let Some(child) = self.get_child() {
            return unsafe { &mut *child }.get_bottom();
        }

        self
    }

    pub fn get_num_of_recurse_components(&mut self) -> usize {
        let mut parent = self as *mut Self;
        let mut num = 0_usize;

        loop {
            if let Some(child) = unsafe { &mut *parent }.get_child() {
                parent = child;

                num += 1;
            } else {
                break;
            }
        }

        num
    }

    pub fn get_element(&mut self, element: usize) -> Option<*mut Self> {
        let mut parent = self as *mut Self;
        let mut num_remaining = element;

        loop {
            if num_remaining == 0 {
                return Some(parent);
            }

            if let Some(child) = unsafe { &mut *parent }.get_child() {
                parent = child;
                num_remaining -= 1;
            } else {
                break;
            }
        }

        None
    }
}

pub struct ByteVec<'a, T> {
    parent: Option<RecursiveComponent<'a, T>>
}

impl<'a, T> ByteVec<'a, T> {
    pub fn new() -> Self {
        Self {
            parent: None
        }
    }

    pub fn add_bytes(&mut self, bytes: &'a mut [u8]) -> Result<(), QuantumError> {
        if self.parent.is_none() {
            let component = RecursiveComponent::<T>::new(bytes)?;
            self.parent = Some(component);

            return Ok(());
        }

        if let Some(parent) = &mut self.parent {
            // finally we found a parent without a child, so lets add one
            let child = RecursiveComponent::<T>::new(bytes)?;
            let bottom_ref = unsafe { &mut *parent.get_bottom() };

            bottom_ref.recurse_next_component(child)?;

            return Ok(());
        }

        Err(QuantumError::UndefinedValue)
    }

    pub fn push(&mut self, element: T) -> Result<(), QuantumError> {
        if let Some(parent) = &mut self.parent {
            let mut iteration_ref = parent as *mut RecursiveComponent<T>;

            while unsafe { &mut *iteration_ref }.is_full() {
                if let Some(child) = unsafe { &mut *iteration_ref }.get_child(){
                    iteration_ref = child;
                } else {
                    return Err(QuantumError::BufferFull);
                }
            }

            return unsafe { &mut *iteration_ref }.push(element);
        }


        Err(QuantumError::UndefinedValue)
    }

    pub fn remove(&mut self, index: usize) -> Result<(), QuantumError> {
        if let Some(parent) = &mut self.parent {
            let mut iteration_ref = parent as *mut RecursiveComponent<T>;
            let mut total_elements = unsafe { &mut *iteration_ref }.len();
            let mut index_sub = 0;

            while index >= total_elements {
                if let Some(child) = unsafe { &mut *iteration_ref }.get_child() {
                    iteration_ref = child;
                    total_elements += unsafe { &mut *iteration_ref }.len();
                    index_sub += unsafe { &mut *iteration_ref }.total_size();
                } else {
                    return Err(QuantumError::NoItem);
                }
            }

            if index >= total_elements {
                return Err(QuantumError::NoItem);
            }


            unsafe { &mut *iteration_ref }.remove_element(index - index_sub);


            while unsafe { &mut *iteration_ref }.is_parent() {
                if let Some(child) = unsafe { &mut *iteration_ref }.get_child() {
                    if let Ok(value) = unsafe { &mut *child }.get(0) {

                        let value = unsafe {
                            core::ptr::read_unaligned(value)
                        };

                        unsafe { &mut *child }.remove_element(0);
                        unsafe { &mut *iteration_ref }.push(value)?;

                        iteration_ref = child;
                    } else {
                        break;
                    }
                }
            }

            return Ok(());
        }

        Err(QuantumError::UndefinedValue)
    }

    pub fn get(&mut self, index: usize) -> Result<&mut T, QuantumError> {
        if let Some(parent) = &mut self.parent {
            let mut iteration_ref = parent as *mut RecursiveComponent<T>;
            let mut total_elements = unsafe { &mut *iteration_ref }.len();
            let mut index_sub = 0;

            // here we are trying to get the component that contains the element we are looking for
            // this is important because our parent element is likely the component of the element.
            while index >= total_elements {
                if let Some(child) = unsafe { &mut *iteration_ref }.get_child() {
                    iteration_ref = child;
                    index_sub += unsafe { &mut *iteration_ref }.total_size();
                    total_elements += unsafe { &mut *iteration_ref }.len();
                } else {
                    return Err(QuantumError::NoItem);
                }
            }

            if index >= total_elements {
                return Err(QuantumError::NoItem);
            }

            return unsafe { &mut *iteration_ref }.get(index - index_sub);
        }

        Err(QuantumError::UndefinedValue)
    }

    pub fn total_size(&mut self) -> usize {
        if let Some(parent) = &mut self.parent {
            let mut iteration_ref = parent as *mut RecursiveComponent<T>;
            let mut total_size = parent.total_size();

            loop {
                if let Some(child) = unsafe {&mut *iteration_ref}.get_child() {
                    iteration_ref = child;

                    total_size += unsafe { &mut *child }.total_size()
                } else {
                    break;
                }
            }

            return total_size;
        }

        0
    }

    pub fn size(&mut self) -> usize {
        if let Some(parent) = &mut self.parent {
            let mut iteration_ref = parent as *mut RecursiveComponent<T>;
            let mut elements_num = parent.len();

            loop {
                if let Some(child) = unsafe {&mut *iteration_ref}.get_child() {
                    iteration_ref = child;

                    elements_num += unsafe { &mut *child }.len()
                } else {
                    break;
                }
            }

            return elements_num;
        }

        0
    }
}

#[cfg(test)]
mod test_case {
    use crate::memory_utils::byte_vec::{ByteVec, RecursiveComponent};

    #[test_case]
    fn test_constructing_component() {
        let mut limited_lifetime_value = [0_u8; 4096];
        let _component =
            RecursiveComponent::<u8>::new(&mut limited_lifetime_value)
                .expect("Could not construct vector!");
    }

    #[test_case]
    fn test_pushing_to_component() {
        let mut limited_lifetime_value = [0_u8; 4096];
        let mut component =
            RecursiveComponent::<u8>::new(&mut limited_lifetime_value)
                .expect("Could not construct vector!");

        component.push(10).expect("Could not push back value");
        assert_eq!(*component.get(0).unwrap(), 10_u8);
    }

    #[test_case]
    fn test_pushing_many_elements() {
        let mut limited_lifetime_value = [0_u8; 4096];
        let mut component =
            RecursiveComponent::<u8>::new(&mut limited_lifetime_value)
                .expect("Could not construct ");

        for i in 0..component.total_size() {
            component.push(i as u8).expect("Unable to push element");
            assert_eq!(*component.get(i).unwrap(), i as u8);
            assert_eq!(component.len(), i + 1);
        }

        assert_eq!(component.len(), component.total_size());

        for i in 0..component.total_size() {
            assert_eq!(*component.get(i).unwrap(), i as u8);
        }
    }

    #[test_case]
    fn test_push_and_remove_elements() {
        let mut limited_lifetime_value = [0_u8; 4096];
        let mut component =
            RecursiveComponent::<u8>::new(&mut limited_lifetime_value)
                .expect("Could not construct ");

        assert_eq!(component.get_num_of_recurse_components(), 0);

        for i in 0..10 {
            component.push(i as u8).expect("Unable to push element!");
            assert_eq!(component.len(), i + 1);
        }

        for i in 0..6 {
            component.remove_element(i);
            assert_eq!(component.len(), 9 - i);
        }

        for i in 0..4 {
            let check = (i * 2) + 1;

            assert_eq!(*component.get(i).unwrap(), check as u8);
        }

        assert_eq!(component.len(), 4);
    }

    #[test_case]
    fn test_different_sized_element() {
        let mut limited_lifetime_value = [0_u8; 4096];
        let mut component =
            RecursiveComponent::<u64>::new(&mut limited_lifetime_value)
                .expect("Could not construct ");


        for i in 0..component.total_size() {
            component.push(i as u64).expect("Unable to push element");

            assert_eq!(*component.get(i).unwrap(), i as u64);
        }

        assert_eq!(component.is_full(), true);
    }

    #[test_case]
    fn test_recursive_element() {
        let mut limited_lifetime_value = [0_u8; 4096];
        let mut component =
            RecursiveComponent::<u64>::new(&mut limited_lifetime_value)
                .expect("Could not construct ");

        let mut child_buffer = [0_u8; 4096];
        let child =
            RecursiveComponent::<u64>::new(&mut child_buffer)
                .expect("Could not construct ");

        for i in 0..component.total_size() {
            component.push(i as u64).unwrap();
            assert_eq!(component.len(), i + 1);
            assert_eq!(*component.get(i).unwrap(), i as u64);
        }

        assert_eq!(component.is_parent(), false);

        component.recurse_next_component(child).unwrap();

        assert_eq!(component.is_parent(), true);

        let test_child = component.get_child().unwrap();

        for i in 0..unsafe {&mut *test_child}.total_size() {
            unsafe { &mut *test_child }.push(i as u64).unwrap();
            assert_eq!(*(unsafe { &mut *test_child}).get(i).unwrap(), i as u64);
            assert_eq!((unsafe { &mut *test_child}).len(), i + 1);
        }

        for i in 0..component.total_size() {
            assert_eq!(*component.get(i).unwrap(), i as u64);
        }

        for i in 0..unsafe {&mut *test_child}.total_size() {
            assert_eq!(*(unsafe { &mut *test_child}).get(i).unwrap(), i as u64);
        }


    }

    #[test_case]
    fn test_adding_bytes() {
        let mut vector = ByteVec::<u8>::new();

        let mut limited_lifetime_value0 = [0_u8; 4096];
        let mut limited_lifetime_value1 = [0_u8; 4096];
        let mut limited_lifetime_value2 = [0_u8; 4096];

        vector.add_bytes(&mut limited_lifetime_value0).expect("Could not add bytes");
        vector.add_bytes(&mut limited_lifetime_value1).expect("Could not add bytes");
        vector.add_bytes(&mut limited_lifetime_value2).expect("Could not add bytes");

    }

    #[test_case]
    fn test_adding_elements() {
        let mut vector = ByteVec::<u8>::new();

        let mut limited_lifetime_value0 = [0_u8; 196];

        vector.add_bytes(&mut limited_lifetime_value0).expect("Could not add bytes");

        vector.push(10).expect("Unable to push element to vector");
        assert_eq!(*vector.get(0).expect("Unable to get element"), 10);

    }

    #[test_case]
    fn test_adding_elements_to_recurse_elements() {
        let mut vector = ByteVec::<usize>::new();

        let mut limited_lifetime_value0 = [0_u8; 196];
        let mut limited_lifetime_value1 = [0_u8; 196];
        let mut limited_lifetime_value2 = [0_u8; 196];

        vector.add_bytes(&mut limited_lifetime_value0).expect("Could not add bytes");
        vector.add_bytes(&mut limited_lifetime_value1).expect("Could not add bytes");
        vector.add_bytes(&mut limited_lifetime_value2).expect("Could not add bytes");

        for i in 0..vector.total_size() {
            vector.push(i).expect("Unable to push element to vector");
            assert_eq!(*vector.get(i).expect("Unable to get element"), i);
        }

        for _i in 0..vector.total_size() {
            vector.remove(0).expect("Unable to remove element from vector");
        }

        assert_eq!(vector.size(), 0);
    }

    #[test_case]
    fn test_getting_recurse_elements() {
        let mut vector = ByteVec::<u8>::new();

        let mut limited_lifetime_value0 = [0_u8; 196];
        let mut limited_lifetime_value1 = [0_u8; 196];
        let mut limited_lifetime_value2 = [0_u8; 196];
        let mut limited_lifetime_value3 = [0_u8; 196];
        let mut limited_lifetime_value4 = [0_u8; 196];

        vector.add_bytes(&mut limited_lifetime_value0).expect("Could not add bytes");
        vector.add_bytes(&mut limited_lifetime_value1).expect("Could not add bytes");
        vector.add_bytes(&mut limited_lifetime_value2).expect("Could not add bytes");
        vector.add_bytes(&mut limited_lifetime_value3).expect("Could not add bytes");
        vector.add_bytes(&mut limited_lifetime_value4).expect("Could not add bytes");



    }


}