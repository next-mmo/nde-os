import gradio as gr
import os
import sys
import json
import platform

def get_sandbox_info():
    info = {
        "Platform": platform.system(),
        "HOME": os.environ.get("HOME", os.environ.get("USERPROFILE", "unknown")),
        "SANDBOX_ROOT": os.environ.get("SANDBOX_ROOT", "unknown"),
        "TMPDIR": os.environ.get("TMPDIR", os.environ.get("TEMP", "unknown")),
        "PYTHONPATH": os.environ.get("PYTHONPATH", "unknown"),
        "Working Dir": os.getcwd(),
        "Python": sys.executable,
    }
    return json.dumps(info, indent=2)

def test_file_write(filename, content):
    try:
        filepath = os.path.join(os.getcwd(), "data", filename)
        with open(filepath, "w") as f:
            f.write(content)
        return "SUCCESS: Wrote to " + filepath
    except Exception as e:
        return "BLOCKED: " + str(e)

def test_escape(path):
    try:
        real_path = os.path.realpath(path)
        sandbox_root = os.environ.get("SANDBOX_ROOT", "")
        if sandbox_root and not real_path.startswith(sandbox_root):
            return "BLOCKED by sandbox: " + real_path + " outside " + sandbox_root
        if os.path.exists(path):
            return "Path exists: " + real_path
        return "Path not found: " + real_path
    except Exception as e:
        return "Error: " + str(e)

def list_workspace():
    result = []
    cwd = os.getcwd()
    for root, dirs, files in os.walk(cwd):
        level = root.replace(cwd, "").count(os.sep)
        indent = "  " * level
        result.append(indent + "[DIR] " + os.path.basename(root) + "/")
        for f in files:
            size = os.path.getsize(os.path.join(root, f))
            result.append("  " * (level+1) + "[FILE] " + f + " (" + str(size) + "b)")
    return "\n".join(result) if result else "Empty"

with gr.Blocks(title="Sandboxed AI App", theme=gr.themes.Soft()) as app:
    gr.Markdown("## Sandboxed AI App Demo")
    gr.Markdown("Running inside a filesystem jail on **" + platform.system() + "**")
    with gr.Tab("Sandbox Info"):
        gr.Button("Show Environment", variant="primary").click(get_sandbox_info, outputs=gr.Code(language="json"))
    with gr.Tab("Write Test"):
        fn = gr.Textbox(label="Filename", value="test.txt")
        fc = gr.Textbox(label="Content", value="Hello from sandbox!")
        gr.Button("Write", variant="primary").click(test_file_write, [fn,fc], gr.Textbox(label="Result"))
    with gr.Tab("Escape Test"):
        ep = gr.Textbox(label="Path", value="/etc/passwd" if platform.system()!="Windows" else "C:\\Windows\\System32\\config\\SAM")
        gr.Button("Test", variant="stop").click(test_escape, [ep], gr.Textbox(label="Result"))
    with gr.Tab("Workspace"):
        gr.Button("List Files", variant="primary").click(list_workspace, outputs=gr.Code())

app.launch(server_name="0.0.0.0", server_port=7860, share=False)
