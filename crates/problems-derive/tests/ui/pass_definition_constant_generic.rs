use issues::Problem;

#[derive(Debug, thiserror::Error, problems_derive::Problem)]
#[problem(prefix = "urn:test")]
enum Failure<T> {
    #[error("failure")]
    #[problem(409)]
    NameConflict { value: T },
}

fn main() {
    assert_eq!(Failure::<std::io::Error>::NAME_CONFLICT.status, 409);
    let error = Failure::NameConflict { value: 1 };
    assert_eq!(error.definition(), &Failure::<i32>::NAME_CONFLICT);
    assert_eq!(Failure::<i32>::definitions().next().unwrap(), error.definition());
}
