# Brainfuck 解释器使用指南

## 项目结构

```
brainfuck/
├── src/
│   ├── lib.rs      # 核心库代码（Memory、Expr枚举、Interpreter等）
│   └── main.rs     # 默认主程序（使用枚举实现）
├── bin/
│   ├── enum_version.rs   # 枚举版本的独立二进制
│   └── trait_version.rs  # Trait版本的独立二进制
├── bench.rs        # 性能基准测试程序
└── *.bf           # Brainfuck源代码文件
```

## 使用方法

### 1. 运行默认解释器（枚举版本）

```bash
cargo run hello.bf
# 或者指定二进制
cargo run --bin brainfuck hello.bf
```

### 2. 运行枚举版本

```bash
cargo run --bin enum_version hello.bf
```

### 3. 运行 Trait 版本

```bash
cargo run --bin trait_version hello.bf
```

### 4. 性能基准测试

编译基准测试程序：
```bash
rustc --edition 2021 -O bench.rs -o bench.exe
```

运行基准测试：
```bash
./bench.exe <bf_file> [iterations]
```

示例：
```bash
./bench.exe hello.bf 100    # 测试 hello.bf，运行100次取平均值
./bench.exe 2.bf 50         # 测试 2.bf，运行50次取平均值
```

## 编译选项

### 开发模式
```bash
cargo build
```

### 发布模式（优化）
```bash
cargo build --release
```

### 单独编译某个二进制
```bash
cargo build --release --bin enum_version
cargo build --release --bin trait_version
```

## 基准测试输出说明

基准测试会显示：
- 解析和执行时间
- 两种实现的性能对比
- 性能差异百分比
- 性能总结

示例输出：
```
Benchmarking Brainfuck Interpreter Implementations
=================================================
File: hello.bf
Iterations: 50

Results
=======
Enum implementation:     6777734 ns (6777.734 μs)
Trait implementation:    6792722 ns (6792.722 μs)

📊 Enum is 1.00x faster than trait
⚡ Performance improvement: 0.2%

Summary
=======
⚖️  Both implementations have similar performance
```

## 可用的 Brainfuck 程序

- `hello.bf` - Hello World 程序
- `1.bf` - 复杂程序示例
- `2.bf` - 另一个测试程序
- `3.bf`、`4.bf`、`5.bf` - 其他测试程序

## 库使用

如果想在自己的项目中使用这个解释器：

```rust
use brainfuck::{Interpreter, Memory};

fn main() {
    let code = "+++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.";
    let mut interpreter = Interpreter::new(code.to_string());
    interpreter.run();
}
```

## 性能优化

项目使用以下编译优化：
- `opt-level = 3` - 最高优化级别
- `lto = true` - 链接时优化
- `codegen-units = 1` - 单一代码生成单元
- `panic = "abort"` - 减少panic处理开销

## 实现差异

### 枚举版本 (enum_version)
- 使用 `Expr` 枚举和 `match` 语句
- 所有指令类型在编译时已知
- 更好的编译器优化
- 内存布局紧凑

### Trait 版本 (trait_version)
- 使用 `Instruction` trait 和动态分发
- 每个指令是独立的结构体
- 运行时多态
- 更灵活但可能有性能开销

基准测试可以帮助你了解在不同场景下哪种实现更适合。