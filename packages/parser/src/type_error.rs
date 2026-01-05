use crate::types::Type;

#[derive(Debug, Clone, PartialEq)]
pub enum TypeError {
	PercentageMustBeNumber,
	TypesTooDistantToBeMergedAndInferred(Box<Type>, Box<Type>),
}
