// 示范结构体
/// 文档注释：一个简单的结构体
pub struct ExampleStruct {
    pub field: i32,
}

// 示范枚举
/// 文档注释：一个简单的枚举
pub enum ExampleEnum {
    VariantOne,
    VariantTwo,
}

// 示范特质
/// 文档注释：一个简单的特质
pub trait ExampleTrait {
    fn example_method(&self);
}

// 示范特质实现（impl Trait for Type）
/// 文档注释：特质实现
impl ExampleTrait for ExampleStruct {
    fn example_method(&self) {
        // 方法实现
    }
}

// 示范类型实现（impl Type）
/// 文档注释：类型实现
impl ExampleStruct {
    pub fn new() -> Self {
        ExampleStruct { field: 0 }
    }
}

// 示范函数
/// 文档注释：一个简单的函数
pub fn example_function() -> i32 {
    42
}

// 示范宏定义
/// 文档注释：一个简单的宏
#[macro_export]
macro_rules! example_macro {
    () => {
        println!("这是一个宏示例");
    };
}

// 示范属性宏定义
/// 文档注释：一个简单的属性宏
/// 第二行注释
/// 第三行
#[proc_macro_attribute]
pub fn example_attribute_macro(args: TokenStream, input: TokenStream) -> TokenStream {
    // 宏实现
}

// 示范类型别名
/// 文档注释：一个简单的类型别名
pub type ExampleTypeAlias = i32;
