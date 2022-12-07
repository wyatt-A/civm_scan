use std::ops;
use std::fmt;
use common_math::rounding::round;
use serde::{Serialize, Deserialize};

#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct Vector{
    pub x:f32,
    pub y:f32,
    pub z:f32
}

impl Vector {
    /** Returns a unit vector along the supplied axis */
    pub fn unit(s:char) -> Vector{
        return match s {
            'x' | 'X' => Vector{x:1.0,y:0.0,z:0.0},
            'y' | 'Y' => Vector{x:0.0,y:1.0,z:0.0},
            'z' | 'Z' => Vector{x:0.0,y:0.0,z:1.0},
            _ => panic!("{} is not a valid direction",s),
        }
    }
    /** Returns a null vector where are fields are 0.0 */
    pub fn null() -> Vector{
        return Vector{x:0.0,y:0.0,z:0.0};
    }

    /** Returns a null vector where are fields are 1.0 */
    pub fn ones() -> Vector {
        return Vector{x:1.0,y:1.0,z:1.0};
    }

    /** Returns a new vector where are fields are defined up-front */
    pub fn new(x:f32,y:f32,z:f32) -> Vector {
        return Vector{x:x,y:y,z:z};
    }
    
    pub fn mag(&self) -> f32{
        return (self.x*self.x + self.y*self.y + self.z*self.z).sqrt();
    }

    /** Print to console */
    pub fn print(&self){
        println!("{}",self.to_str());
    }
    /** Format to string */
    pub fn to_str(&self) -> String{
        return format!("{},{},{}",self.x,self.y,self.z);
    }

    pub fn mul_entries(&self,v:&Vector) -> Vector{
        return Vector::new(self.x*v.x,self.y*v.y,self.z*v.z);
    }

}

/** Vector addition */
impl ops::Add<Vector> for Vector {
    type Output = Vector;
    fn add(self, _rhs:Vector) -> Vector {
        return Vector::new(self.x + _rhs.x,self.y + _rhs.y,self.z + _rhs.z);
    }
}

/** Vector subtraction */
impl ops::Sub<Vector> for Vector {
    type Output = Vector;
    fn sub(self, _rhs:Vector) -> Vector {
        return Vector::new(self.x - _rhs.x,self.y - _rhs.y,self.z - _rhs.z);
    }
}

/** Vector-scalar subtraction */
impl ops::Sub<f32> for Vector {
    type Output = Vector;
    fn sub(self, _rhs:f32) -> Vector {
        return Vector::new(self.x - _rhs,self.y - _rhs,self.z - _rhs);
    }
}

/** Vector scalar division */
impl ops::Div<f32> for Vector {
    type Output = Vector;
    fn div(self, _rhs:f32) -> Vector {
        if _rhs == 0.0{panic!("cannot divide vector by zero");}
        return Vector::new(self.x/_rhs,self.y/_rhs,self.z/_rhs);
    }
}

/** Vector dot product */
impl ops::Mul<Vector> for Vector {
    type Output = f32;
    fn mul(self, _rhs:Vector) -> f32 {
        return self.x*_rhs.x + self.y*_rhs.y + self.z*_rhs.z;
    }
}

/** Vector-scalar multiplication */
impl ops::Mul<Vector> for f32 {
    type Output = Vector;
    fn mul(self,_rhs:Vector) -> Vector {
        return Vector::new(self*_rhs.x, self*_rhs.y, self*_rhs.z);
    }
}

/** Vector-scalar multiplication (commutative) */
impl ops::Mul<f32> for Vector {
    type Output = Vector;
    fn mul(self,_rhs:f32) -> Vector {
        return Vector::new(self.x*_rhs, self.y*_rhs, self.z*_rhs);
    }
}


#[derive(Copy, Clone, Serialize, Deserialize)]
pub struct Matrix{
    pub a:Vector,
    pub b:Vector,
    pub c:Vector
}

impl Matrix{

    /** Returns a new matrix where are fields are defined up-front */
    pub fn new(a:Vector,b:Vector,c:Vector) -> Matrix{
        return Matrix{a:a,b:b,c:c};
    }
    /** Return a matrix where are fields are 1.0 */
    pub fn ones() -> Matrix{
        return Matrix{a:Vector::ones(),b:Vector::ones(),c:Vector::ones()};
    }
    /** Returns the identity matrix */
    pub fn identity() -> Matrix{
        return Matrix{
            a:Vector::unit('x'),
            b:Vector::unit('y'),
            c:Vector::unit('z')
        }
    }
    /** Returns Matrix of all zeros */
    pub fn zeros() -> Matrix{
        return Matrix{
            a:Vector::null(),
            b:Vector::null(),
            c:Vector::null()
        }
    }

}

/** Matrix-vector multiplication */
impl ops::Mul<Vector> for Matrix {
    type Output = Vector;
    fn mul(self, _rhs:Vector) -> Vector {
        return Vector{x:self.a*_rhs,y:self.b*_rhs,z:self.c*_rhs};
    }
}

impl ops::Mul<f32> for Matrix {
    type Output = Matrix;
    fn mul(self,_rhs:f32) -> Matrix{
        return Matrix{a:self.a*_rhs,b:self.b*_rhs,c:self.c*_rhs};
    }
}

impl ops::Mul<Matrix> for f32 {
    type Output = Matrix;
    fn mul(self,_rhs:Matrix) -> Matrix{
        return Matrix{a:self*_rhs.a,b:self*_rhs.b,c:self*_rhs.c};
    }
}

impl fmt::Display for Vector {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        write!(f, "{},{},{}", self.x,self.y,self.z)
    }
}

impl fmt::Debug for Vector {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Vector")
            .field("x", &self.x)
            .field("y", &self.y)
            .field("z", &self.z)
            .finish()
    }
}

/** Determines precision conditions for equality to 5 decimal places */
impl PartialEq for Vector {
    fn eq(&self, other: &Self) -> bool {
        let round_to = 5; //decimal places
        let x = round(self.x,round_to) == round(other.x,round_to);
        let y = round(self.y,round_to) == round(other.y,round_to);
        let z = round(self.z,round_to) == round(other.z,round_to);
        return x && y && z;
    }
}

impl fmt::Display for Matrix {
    // This trait requires `fmt` with this exact signature.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Write strictly the first element into the supplied output
        // stream: `f`. Returns `fmt::Result` which indicates whether the
        // operation succeeded or failed. Note that `write!` uses syntax which
        // is very similar to `println!`.
        write!(f, "\n{}\n{}\n{}", self.a,self.b,self.c)
    }
}

impl fmt::Debug for Matrix {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Vector")
            .field("a", &self.a)
            .field("b", &self.b)
            .field("c", &self.c)
            .finish()
    }
}

impl PartialEq for Matrix {
    fn eq(&self, other: &Self) -> bool {
        return self.a == other.a && self.b == other.b && self.c == other.c;
    }
}


#[cfg(test)]
mod tests {
use super::*; // brings parent module into scope (including private functions)

#[test]
fn mat_vec_multiplication() {
    let v = Vector::unit('x');
    let m = Matrix::identity();
    let r = Vector::unit('x');
    println!("{}",v);
    assert_eq!(m*v, r,"asserting equality between {} and {}",m*v,r);
    
    let v = Vector::unit('y');
    let m = Matrix::identity();
    let r = Vector::unit('y');
    println!("{}",v);
    assert_eq!(m*v, r,"asserting equality between {} and {}",m*v,r);

    let v = Vector::unit('z');
    let m = Matrix::identity();
    let r = Vector::unit('z');
    println!("{}",v);
    assert_eq!(m*v, r,"asserting equality between {} and {}",m*v,r);
}

#[test]
fn vec_dot_product(){
    let v = Vector::new(4.0,2.1,3.1);
    let w = Vector::new(0.0,0.0,0.0);
    let r = 0.0;
    assert_eq!(v*w,r,"asserting equality between {} and {}",v*w,r);

    let v = Vector::new(4.0,2.1,3.1);
    let w = Vector::new(2.0,1.0,0.2);
    let r = 10.72;
    assert_eq!(v*w,r,"asserting equality between {} and {}",v*w,r);
}

#[test]
fn vec_addition(){
    let v = Vector::new(4.0,2.1,3.1);
    let w = Vector::new(2.0,1.0,0.2);
    let r = Vector::new(6.0,3.1,3.3);
    assert_eq!(v+w,r,"asserting equality between {} and {}",v+w,r);
    assert_eq!(w+v,r,"asserting equality between {} and {}",w+v,r);
}

#[test]
fn vec_subtraction(){
    let v = Vector::new(4.0,2.1,3.1);
    let w = Vector::new(2.0,1.0,0.2);
    let r1 = Vector::new(2.0,1.1,2.9);
    let r2 = Vector::new(-2.0,-1.1,-2.9);
    assert_eq!(v-w,r1,"asserting equality between {} and {}",v-w,r1);
    assert_eq!(w-v,r2,"asserting equality between {} and {}",w-v,r2);
}

#[test]
fn matrix_equality(){
    let m = Matrix::zeros();
    let r = Matrix::new(Vector::null(),Vector::null(),Vector::new(0.000001,0.0,0.0));
    assert_eq!(m,r,"asserting equality between {} and {}",m,r);
}

#[test]
fn vector_scalar_multiplication(){
    let v = Vector::unit('x');
    let r = Vector::new(2.0,0.0,0.0);
    assert_eq!(2.0*v,r);
}
}