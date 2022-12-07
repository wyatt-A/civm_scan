use crate::matmath::{Vector,Matrix};
// % operator splitting method for stationary spins (Bloch equations)
// % Arijit Hazra, 2016 (Ph.D dissertation)
pub fn rot_op(bvec:&Vector,tau:f32,gamma:f32) -> Matrix{
    let mut result = Matrix::zeros();
    let norm = bvec.mag();
    let n = (1.0/norm)*(*bvec);
    let phi = tau*gamma*norm;
    let cosphi = phi.cos();
    let onemcosphi = 1.0 - cosphi;
    let sinphi = phi.sin();
    let nxsinphi = n.x*sinphi;
    let nysinphi = n.y*sinphi;
    let nzsinphi = n.z*sinphi;
    let nxnx = n.x*n.x;let nyny = n.y*n.y;let nznz = n.z*n.z;
    let nxny = n.x*n.y;let nxnz = n.x*n.z;let nynz = n.y*n.z;
    result.a.x = nxnx + (1.0 - nxnx)*cosphi;
    result.a.y = nxny*(onemcosphi) + nzsinphi;
    result.a.z = nxnz*onemcosphi - nysinphi;
    result.b.x = nxny*onemcosphi - nzsinphi;
    result.b.y = nyny + (1.0-nyny)*cosphi;
    result.b.z = nynz*onemcosphi + nxsinphi;
    result.c.x = nxnz*onemcosphi + nysinphi;
    result.c.y = nynz*onemcosphi - nxsinphi;
    result.c.z = nznz + (1.0 - nznz)*cosphi;
    return result;
}
    
pub fn rel_op(mag:Vector,t1:f32,t2:f32,tau:f32,m0:f32) -> Vector {
    let t_rel = (-tau/t2).exp();
    let l_rel = (-tau/t1).exp();
    let rel = Vector::new(t_rel,t_rel,l_rel);
    let mut m = mag.mul_entries(&rel);
    m.z = m.z + m0*(1.0-l_rel);
    return m;
}

pub fn transform(mag:Vector,b:&Vector,t1:f32,t2:f32,gamma:f32,m0:f32,tau:f32) -> Vector{
    let rotop = rot_op(b,tau,gamma);
    return rel_op(rotop*mag,t1,t2,tau,m0);
}