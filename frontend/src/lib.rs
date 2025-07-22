mod websocket;

use std::cell::RefCell;
use wasm_bindgen::prelude::*;
use yew::{function_component, html, Html};
use futures::channel::mpsc::Sender;

use std::rc::Rc;
use gloo_net::websocket::Message;
use yew::prelude::*;
use crate::websocket::WebsocketService;

#[derive(Debug, Clone, PartialEq)]
pub struct AppStateInner {
    pub message: String,
}

type AppState = UseStateHandle<Rc<AppStateInner>>;


#[derive(Properties)]
pub struct HelloWorldProps {
    pub ws_ref: Rc<WebsocketService>,
}
impl PartialEq for HelloWorldProps {
    fn eq(&self, _other: &Self) -> bool {
        true // o false si quieres forzar siempre un rerender
    }
}


#[function_component]
fn HelloWorld(props: &HelloWorldProps) -> Html {
    let context = use_context::<AppState>().expect("No context found.");
    let mut set_context = context.clone();
    let ws_ref = props.ws_ref.clone();

    let oninput = {
        Callback::from(move |e: web_sys::InputEvent| {
            if let Some(input) = e.target_dyn_into::<web_sys::HtmlInputElement>() {
                let new_message = input.value();
                set_context.set(Rc::new(AppStateInner {message: new_message}));
            }
        })
    };

    let onclick = {
        let msg = context.message.clone();
        Callback::from(move |_| {
            ws_ref.send(msg.clone());
        })
    };

    html! {
        <>
            <h1>{ "Hello world" }</h1>
            <div>
                <input {oninput} value={context.message.clone()} />
                <button {onclick}>{ "Send" }</button>
            </div>
        </>
    }

}

#[function_component]
pub fn App() -> Html {
    /*  Initialize context */
    let ctx = use_state(|| {
        Rc::new(AppStateInner {
            message: String::from("Welcome to WebAssembly!"),
        })
    });

    /*
    {
        let ctx_clone = ctx.clone();
        let ws_ref = use_mut_ref(|| None::<WebsocketService>);

        use_effect(move || {
            *ws_ref.borrow_mut() = Some(WebsocketService::new());

        });
    }
     */
    // App.rs
    let ws = Rc::new(WebsocketService::new());
    html! {
    <ContextProvider<AppState> context={ctx.clone()}>
        <HelloWorld ws_ref={ws.clone()} />
    </ContextProvider<AppState>>
    }

}

// Then somewhere else you can use the component inside `html!`


#[wasm_bindgen(start)]
pub fn start() {
    yew::Renderer::<App>::new().render();
}
