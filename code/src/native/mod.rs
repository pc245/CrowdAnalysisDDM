#[allow(dead_code, unused_variables)]
pub mod opencv {
    use image;
    use libc::{c_char, c_int, size_t};

    #[repr(C)]
    #[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
    #[allow(clippy::enum_variant_names)]
    enum CvType {
        /// 8 bit unsigned, single channel (grey image)
        Cv8UC1 = 0,
        /// 8 bit signed, single channel (grey image)
        Cv8SC1 = 1,
        /// 16 bit unsigned, single channel (grey image)
        Cv16UC1 = 2,
        /// 16 bit signed, single channel (grey image)
        Cv16SC1 = 3,
        /// 32 bit signed, single channel (grey image)
        Cv32SC1 = 4,
        /// 32 bit float, single channel (grey image)
        Cv32FC1 = 5,
        /// 32 bit float, single channel (grey image)
        Cv64FC1 = 6,
        /// 8 bit, two channel (rarely seen)
        Cv8UC2 = 8,
        /// 8 bit unsigned, three channels (RGB image)
        Cv8UC3 = 16,
        /// 8 bit signed, three channels (RGB image)
        Cv8SC3 = 17,
        /// 16 bit unsigned, three channels (RGB image)
        Cv16UC3 = 18,
        /// 16 bit signed, three channels (RGB image)
        Cv16SC3 = 19,
        /// 32 bit signed, three channels (RGB image)
        Cv32SC3 = 20,
        /// 32 bit float, three channels (RGB image)
        Cv32FC3 = 21,
        /// 32 bit float, three channels (RGB image)
        Cv64FC3 = 22,
    }

    extern "C" {
        fn start_capture(s: *const c_char) -> size_t;
        fn start_camera_capture() -> size_t;
        fn get_frame(stream_id: size_t) -> *mut CMat;
        fn mat_new() -> *mut CMat;
        fn mat_cols(cmat: *const CMat) -> c_int;
        fn mat_rows(cmat: *const CMat) -> c_int;
        fn mat_depth(cmat: *const CMat) -> c_int;
        fn mat_channels(cmat: *const CMat) -> c_int;
        fn mat_drop(cmat: *mut CMat);
        fn mat_data(cmat: *const CMat) -> *const u8;
        fn mat_total(cmat: *const CMat) -> usize;
        fn mat_step1(cmat: *const CMat, i: c_int) -> usize;
        fn mat_elem_size(cmat: *const CMat) -> usize;
        fn mat_elem_size1(cmat: *const CMat) -> usize;
        fn mat_type(cmat: *const CMat) -> CvType;
        fn close_stream(stream_id: usize);
    //pub fn write(filename: *const c_char, cmat: *const CMat);
    //pub fn show_next(stream_id: size_t);
    }

    pub fn close_stream_safe(stream_id: usize) {
        unsafe {
            close_stream(stream_id);
        }
    }

    #[derive(Clone, Debug)]
    pub enum CMat {}

    impl CMat {
        pub(crate) fn new() -> *mut CMat {
            unsafe { mat_new() }
        }
    }

    impl Drop for Mat {
        fn drop(&mut self) {
            unsafe {
                mat_drop(self.inner);
            }
        }
    }

    unsafe impl Send for CMat {}
    unsafe impl Send for Mat {}
    // impl Into<CMat> for Mat {
    //     fn into(self) -> CMat {
    //         unsafe { *self.inner }
    //     }
    // }

    #[derive(Debug)]
    pub struct Mat {
        /// Pointer to the actual C/C++ data structure
        pub(crate) inner: *mut CMat,

        /// Number of columns
        pub cols: u64,

        /// Number of rows
        pub rows: u64,

        /// Depth of this mat (it should be the type).
        pub depth: u64,

        /// Channels of this mat
        pub channels: u64,
    }

    pub struct Image {
        pub data: arrayfire::Array<u8>,
        pub channels: u64,
        pub rows: u64,
        pub cols: u64,
        pub depth: u64,
    }

    pub struct GrayImage {
        pub data: arrayfire::Array<u8>,
        pub channels: u64,
        pub rows: u64,
        pub cols: u64,
        pub depth: u64,
    }

    //TODO: maybe store as Dim4(cols, rows,) this does not work
    impl Image {
        pub fn new(frame: &Mat) -> Image {
            let data = frame.data();
            Image {
                data: arrayfire::Array::new(
                    data,
                    arrayfire::Dim4::new(&[frame.cols, frame.rows, frame.channels, 1]),
                ),
                channels: frame.channels,
                rows: frame.rows,
                cols: frame.cols,
                depth: frame.depth,
            }
        }

        pub fn from(arr: arrayfire::Array<u8>) -> Image {
            let dims = arr.dims().get().to_vec();
            Image {
                data: arr,
                channels: dims[2],
                cols: dims[0],
                rows: dims[1],
                depth: 0,
            }
        }

        pub fn get_frame(stream_id: usize) -> Option<Image> {
            let frame = get_frame_safe(stream_id);
            match frame {
                Some(f) => Some(Image::new(&f)),
                None => None,
            }
        }

        pub fn to_buffer(&self) -> image::DynamicImage {
            let mut data: Vec<u8> = vec![0; (self.rows * self.cols * self.channels) as usize];
            self.data.host(data.as_mut_slice());
            let mut buffer = image::ImageBuffer::new(self.cols as u32, self.rows as u32);
            (0..(self.cols * self.rows)).for_each(|index| {
                let mut arr: [u8; 3] = Default::default();
                arr.copy_from_slice(&data[((3 * index) as usize)..((3 * index + 3) as usize)]);
                buffer.put_pixel(
                    ((index as u64) - ((index as f64 / (self.cols as f64)) as u64) * self.cols)
                        as u32,
                    (index as f64 / (self.cols as f64)) as u32,
                    image::Rgb { data: arr },
                );
            });
            image::DynamicImage::ImageRgb8(buffer)
        }
    }

    impl GrayImage {
        pub fn new(frame: &Mat) -> GrayImage {
            let data = frame.data();
            let mut vector: Vec<u8> = Vec::with_capacity((frame.cols * frame.rows) as usize);
            for index in 0..vector.capacity() {
                let (r, g, b) = match &data[3 * index..(3 * index + 3)] {
                    [r, g, b] => (
                        f64::from(*r) / 255.0,
                        f64::from(*g) / 255.0,
                        f64::from(*b) / 255.0,
                    ),
                    _ => (0.0, 0.0, 0.0),
                };
                let greyscale = (0.2126 * r + 0.7152 * g + 0.0722 * b) * (255.0);
                vector.push(greyscale as u8);
            }
            GrayImage {
                data: arrayfire::Array::new(
                    vector.as_slice(),
                    arrayfire::Dim4::new(&[frame.cols, frame.rows, 1, 1]),
                ),
                channels: frame.channels,
                rows: frame.rows,
                cols: frame.cols,
                depth: frame.depth,
            }
        }

        pub fn get_frame(stream_id: usize) -> Option<GrayImage> {
            let frame = get_frame_safe(stream_id);
            match frame {
                Some(f) => Some(GrayImage::new(&f)),
                None => None,
            }
        }

        pub fn from(arr: arrayfire::Array<u8>) -> GrayImage {
            let dims = arr.dims().get().to_vec();
            GrayImage {
                data: arr,
                channels: 3,
                cols: dims[0],
                rows: dims[1],
                depth: 0,
            }
        }

        pub fn to_buffer(&self) -> image::DynamicImage {
            let mut data: Vec<u8> = vec![0u8; (self.rows * self.cols) as usize];
            self.data.host(data.as_mut_slice());
            let mut buffer = image::ImageBuffer::new(self.cols as u32, self.rows as u32);
            data.iter().enumerate().for_each(|(index, &v)| {
                buffer.put_pixel(
                    ((index as u64) - ((index as f64 / (self.cols as f64)) as u64) * self.cols)
                        as u32,
                    (index as f64 / (self.cols as f64)) as u32,
                    image::Luma { data: [v] },
                );
            });
            image::DynamicImage::ImageLuma8(buffer)
        }
    }

    impl Mat {
        #[inline]
        pub(crate) fn from_raw(raw: *mut CMat) -> Mat {
            Mat {
                inner: raw,
                rows: unsafe { mat_rows(raw) as u64 },
                cols: unsafe { mat_cols(raw) as u64 },
                depth: unsafe { mat_depth(raw) as u64 },
                channels: unsafe { mat_channels(raw) as u64 },
            }
        }
        /// Returns the raw data (as a u8 array
        pub fn data(&self) -> &[u8] {
            let bytes = unsafe { mat_data(self.inner) };
            let len = self.total() * self.elem_size();
            unsafe { std::slice::from_raw_parts(bytes, len) }
        }

        pub fn total(&self) -> usize {
            unsafe { mat_total(self.inner) }
        }

        /// Returns the matrix element size in bytes.
        ///
        /// The method returns the matrix element size in bytes. For example, if the
        /// matrix type is CV_16SC3 , the method returns 3*sizeof(short) or 6.
        pub fn elem_size(&self) -> usize {
            unsafe { mat_elem_size(self.inner) }
        }

        /// Returns the size of each matrix element channel in bytes.
        ///
        /// The method returns the matrix element channel size in bytes, that
        /// is, it ignores the number of channels. For example, if the matrix
        /// type is CV_16SC3 , the method returns sizeof(short) or 2.
        pub fn elem_size1(&self) -> usize {
            unsafe { mat_elem_size1(self.inner) }
        }

        /// Returns a normalized step.
        ///
        /// The method returns a matrix step divided by Mat::elemSize1() . It can be
        /// useful to quickly access an arbitrary matrix element
        pub fn step1(&self, i: c_int) -> usize {
            unsafe { mat_step1(self.inner, i) }
        }

        /// Returns the size of this matrix.
        pub fn size(&self) -> (usize, usize) {
            (self.rows as usize, self.cols as usize)
        }
    }

    pub trait CString {
        fn c_string(&self) -> Vec<i8>;
    }

    impl CString for str {
        fn c_string(&self) -> Vec<i8> {
            self.as_bytes()
                .iter()
                .map(|&x| x as i8)
                .collect::<Vec<i8>>()
        }
    }

    pub fn start_capture_safe(s: &str) -> usize {
        let c_string = s.c_string();
        unsafe { start_capture(c_string.as_ptr()) }
    }

    pub fn start_camera_capture_safe() -> usize {
        unsafe { start_camera_capture() }
    }
    //not done
    pub fn get_frame_safe(stream_id: usize) -> Option<Mat> {
        let frame = unsafe { get_frame(stream_id) };
        if frame.is_null() {
            None
        } else {
            Some(Mat::from_raw(frame))
        }
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    // use super::*;

    // #[test]
    // fn test_add() {
    //     assert_eq!(add_safe(1, 2), 3);
    // }
}