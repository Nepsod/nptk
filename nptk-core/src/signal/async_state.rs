#[derive(Clone, Debug, PartialEq)]
pub enum AsyncState<T> {
    Loading,
    Ready(T),
    Error(String),
}

impl<T> Default for AsyncState<T> {
    fn default() -> Self {
        Self::Loading
    }
}

impl<T> AsyncState<T> {
    pub fn is_loading(&self) -> bool {
        matches!(self, Self::Loading)
    }

    pub fn is_ready(&self) -> bool {
        matches!(self, Self::Ready(_))
    }

    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }

    pub fn unwrap(self) -> T {
        match self {
            Self::Ready(val) => val,
            _ => panic!("AsyncState is not ready"),
        }
    }
    
    pub fn as_ref(&self) -> AsyncState<&T> {
        match self {
            Self::Loading => AsyncState::Loading,
            Self::Ready(val) => AsyncState::Ready(val),
            Self::Error(err) => AsyncState::Error(err.clone()),
        }
    }
}
