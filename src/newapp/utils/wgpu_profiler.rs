use wgpu_profiler::GpuTimerQueryResult;

pub fn print_wgpu_profiler_result(res: Vec<GpuTimerQueryResult>) {
    let mut result = "".to_owned();
    print_inner(&mut result, 0, res);
    println!("{result}");
}

fn print_inner(str: &mut String, depth: u32, res: Vec<GpuTimerQueryResult>) {
    res.into_iter().to_owned().for_each(|x| {
        let time = if let Some(time) = x.time {
            (time.end - time.start) * 1000.0
        } else {
            0.0
        };
        let label = x.label;
        let formatted = format!("{label}={time}");
        let indent = "  ".repeat(depth as usize);
        str.push_str(&format!("{indent}{formatted}\n"));
        print_inner(str, depth + 1, x.nested_queries);
    });
}
