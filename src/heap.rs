pub struct Heap<T: std::cmp::PartialEq + std::cmp::PartialOrd> {
    data: Vec<T>,
}

impl<T> Heap<T>
where
    T: std::fmt::Debug + std::cmp::PartialEq + std::cmp::PartialOrd,
{
    pub fn build_max_heap(mut data: Vec<T>) -> Self {
        let l = data.len();
        for i in (0..l / 2).rev() {
            Self::sift_down(&mut data, i, l - 1);
        }
        for i in 0..l - 1 {
            data.swap(0, l - 1 - i);
            Self::sift_down(&mut data, 0, l - 1 - i);
        }
        Self { data }
    }

    fn sift_down(data: &mut [T], start: usize, end: usize) {
        let mut i = start;
        while i < end {
            let mut smallest = i;
            if 2 * i + 1 < end && data[smallest] > data[2 * i + 1] {
                smallest = 2 * i + 1;
            }
            if 2 * i + 2 < end && data[smallest] > data[2 * i + 2] {
                smallest = 2 * i + 2;
            }
            if smallest == i {
                return;
            }
            data.swap(smallest, i);
            i = smallest;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Heap;

    #[test]
    fn test_max_heap() {
        let data = vec![3, 1, 2, 4, 5, 6, 7];
        let heap = Heap::build_max_heap(data);
        assert_eq!(vec![7, 6, 5, 4, 3, 2, 1], heap.data);
    }
}
