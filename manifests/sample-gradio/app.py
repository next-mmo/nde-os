"""
Sample Counter — AI Launcher
A simple Gradio counter app to verify the sandbox is working.
"""
import gradio as gr

counter = 0


def increment():
    global counter
    counter += 1
    return str(counter)


def decrement():
    global counter
    counter -= 1
    return str(counter)


def reset():
    global counter
    counter = 0
    return str(counter)


with gr.Blocks(title="Sample Counter", theme=gr.themes.Soft()) as demo:
    gr.Markdown("# 🔢 Sample Counter\nRunning inside the **AI Launcher** sandbox.")

    display = gr.Textbox(label="Counter", value="0", interactive=False)

    with gr.Row():
        btn_dec = gr.Button("➖ Decrement", variant="secondary")
        btn_reset = gr.Button("🔄 Reset")
        btn_inc = gr.Button("➕ Increment", variant="primary")

    btn_inc.click(fn=increment, outputs=display)
    btn_dec.click(fn=decrement, outputs=display)
    btn_reset.click(fn=reset, outputs=display)

if __name__ == "__main__":
    demo.launch(server_name="0.0.0.0", server_port=7860)
