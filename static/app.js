let activePort = 3000;

const portElements = {
    display: document.getElementById('port-display'),
    node: document.getElementById('node-port'),
    step: document.getElementById('step-port'),
    cta: document.getElementById('cta-port'),
};

function updatePort(port) {
    activePort = port;
    Object.values(portElements).forEach((element) => {
        if (element) element.textContent = port;
    });
}

document.getElementById('port-tabs')?.addEventListener('click', (event) => {
    const button = event.target.closest('.port-tab');
    if (!button) return;

    document.querySelectorAll('.port-tab').forEach((tab) => tab.classList.remove('active'));
    button.classList.add('active');

    const customPortInput = document.getElementById('custom-port');
    if (customPortInput) customPortInput.value = '';

    updatePort(button.dataset.port);
});

document.getElementById('custom-port')?.addEventListener('input', (event) => {
    const port = parseInt(event.target.value);
    if (port >= 1 && port <= 65535) {
        document.querySelectorAll('.port-tab').forEach((tab) => tab.classList.remove('active'));
        updatePort(port);
    }
});

function copyCommand() {
    const command = `ssh -oStrictHostKeyChecking=no -NR 80:localhost:${activePort} t.tn3w.dev`;
    navigator.clipboard.writeText(command).catch(() => {});

    const button = document.getElementById('copy-btn');
    if (!button) return;

    button.textContent = 'Copied!';
    button.classList.add('copied');

    setTimeout(() => {
        button.textContent = 'Copy';
        button.classList.remove('copied');
    }, 2000);
}

document.getElementById('copy-btn')?.addEventListener('click', copyCommand);
document.getElementById('cta-cmd')?.addEventListener('click', copyCommand);
