use leptos::*;
use phosphor_leptos::PlayCircle;

fn main() {
    console_error_panic_hook::set_once();

    mount_to_body(|| {
        view! { <App/> }
    })
}

#[component]
fn App() -> impl IntoView {
    let (clicked, set_clicked) = create_signal(false);
    view! {
        <Show
            when=move || clicked.get()
            fallback=move || {
                view! {
                    <div class="flex w-full items-center justify-center h-screen">
                        <button class="btn" on:click=move |_| set_clicked.set(true)>
                            <PlayCircle size="32px"/>
                            "Play Game"
                        </button>
                    </div>
                }
            }
        >

            <iframe src="./game/index.html" class="w-full h-screen"></iframe>
        </Show>
    }
}
