pub fn m_number_formatter(n_elements:usize) -> Vec<String>{
    let w = ((n_elements-1) as f32).log10().floor() as usize + 1;
    let formatter = |index:usize| format!("m{:0width$ }",index,width=w);
    (0..n_elements).map(|index| formatter(index)).collect()
}
