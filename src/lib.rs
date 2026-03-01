#![cfg_attr(windows, feature(abi_vectorcall))]

use ext_php_rs::prelude::*;
use colorize::AnsiColor;

#[php_class]
#[derive(Debug,Default)]
struct Colorize {
    inner: String
}

#[php_impl]
impl Colorize {
    pub fn __construct(value: String) -> Self {
        Colorize {
            inner: value
        }
    }
    
    fn __to_string(&self) -> String {
        self.inner.clone()
    }

    fn black(&self) -> Self {
        Self {
            inner: self.inner.clone().clone().black()
        }
    }

    fn red(&self) -> Self {
        Self {
            inner: self.inner.clone().red()
        }
    }

    fn green(&self) -> Self {
        Self {
            inner: self.inner.clone().green()
        }
    }

    fn yellow(&self) -> Self {
        Self {
            inner: self.inner.clone().yellow()
        }
    }

    fn blue(&self) -> Self {
        Self {
            inner: self.inner.clone().blue()
        }
    }

    fn magenta(&self) -> Self {
        Self {
            inner: self.inner.clone().magenta()
        }
    }

    fn cyan(&self) -> Self {
        Self {
            inner: self.inner.clone().cyan()
        }
    }

    fn grey(&self) -> Self {
        Self {
            inner: self.inner.clone().grey()
        }
    }

    fn b_black(&self) -> Self {
        Self {
            inner: self.inner.clone().b_black()
        }
    }

    fn b_red(&self) -> Self {
        Self {
            inner: self.inner.clone().b_red()
        }
    }

    fn b_green(&self) -> Self {
        Self {
            inner: self.inner.clone().b_green()
        }
    }

    fn b_yellow(&self) -> Self {
        Self {
            inner: self.inner.clone().b_yellow()
        }
    }

    fn b_blue(&self) -> Self {
        Self {
            inner: self.inner.clone().b_blue()
        }
    }

    fn b_magenta(&self) -> Self {
        Self {
            inner: self.inner.clone().b_magenta()
        }
    }

    fn b_cyan(&self) -> Self {
        Self {
            inner: self.inner.clone().b_cyan()
        }
    }

    fn b_grey(&self) -> Self {
        Self {
            inner: self.inner.clone().b_grey()
        }
    }

    fn default(&self) -> Self {
        Self {
            inner: self.inner.clone().default()
        }
    }

    fn blackb(&self) -> Self {
        Self {
            inner: self.inner.clone().blackb()
        }
    }

    fn redb(&self) -> Self {
        Self {
            inner: self.inner.clone().redb()
        }
    }

    fn greenb(&self) -> Self {
        Self {
            inner: self.inner.clone().greenb()
        }
    }

    fn yellowb(&self) -> Self {
        Self {
            inner: self.inner.clone().yellowb()
        }
    }

    fn blueb(&self) -> Self {
        Self {
            inner: self.inner.clone().blueb()
        }
    }

    fn magentab(&self) -> Self {
        Self {
            inner: self.inner.clone().magentab()
        }
    }

    fn cyanb(&self) -> Self {
        Self {
            inner: self.inner.clone().cyanb()
        }
    }

    fn greyb(&self) -> Self {
        Self {
            inner: self.inner.clone().greyb()
        }
    }

    fn b_blackb(&self) -> Self {
        Self {
            inner: self.inner.clone().b_blackb()
        }
    }

    fn b_redb(&self) -> Self {
        Self {
            inner: self.inner.clone().b_redb()
        }
    }

    fn b_greenb(&self) -> Self {
        Self {
            inner: self.inner.clone().b_greenb()
        }
    }

    fn b_yellowb(&self) -> Self {
        Self {
            inner: self.inner.clone().b_yellowb()
        }
    }

    fn b_blueb(&self) -> Self {
        Self {
            inner: self.inner.clone().b_blueb()
        }
    }

    fn b_magentab(&self) -> Self {
        Self {
            inner: self.inner.clone().b_magentab()
        }
    }

    fn b_cyanb(&self) -> Self {
        Self {
            inner: self.inner.clone().b_cyanb()
        }
    }

    fn b_greyb(&self) -> Self {
        Self {
            inner: self.inner.clone().b_greyb()
        }
    }

    fn defaultb(&self) -> Self {
        Self {
            inner: self.inner.clone().defaultb()
        }
    }

    fn underlined(&self) -> Self {
        Self {
            inner: self.inner.clone().underlined()
        }
    }

    fn bold(&self) -> Self {
        Self {
            inner: self.inner.clone().bold()
        }
    }

    fn blink(&self) -> Self {
        Self {
            inner: self.inner.clone().blink()
        }
    }

    fn reverse(&self) -> Self {
        Self {
            inner: self.inner.clone().reverse()
        }
    }

    fn concealed(&self) -> Self {
        Self {
            inner: self.inner.clone().concealed()
        }
    }

    fn faint(&self) -> Self {
        Self {
            inner: self.inner.clone().faint()
        }
    }

    fn italic(&self) -> Self {
        Self {
            inner: self.inner.clone().italic()
        }
    }

    fn crossedout(&self) -> Self {
        Self {
            inner: self.inner.clone().crossedout()
        }
    }
    
}
#[php_module]
pub fn get_module(module: ModuleBuilder) -> ModuleBuilder {
    module.class::<Colorize>()
}
