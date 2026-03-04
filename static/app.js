let activePort = 3000;

const portElements = {
    display: document.getElementById('port-display'),
    node: document.getElementById('node-port'),
    step: document.getElementById('step-port'),
    cta: document.getElementById('cta-port'),
    explain: document.getElementById('explain-port'),
    aliasDemo: document.getElementById('alias-port-demo'),
    aliasDemo2: document.getElementById('alias-port-demo2'),
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

const INSTALL_COMMANDS = {
    bash: `sed -i '/^qtnl()/d' ~/.bashrc
echo \\
  'qtnl(){ ssh "\${2:-user}"@t.tn3w.dev -oStrictHostKeyChecking=no -NR 80:localhost:"\${1:-3000}"; }' \\
  >> ~/.bashrc`,
    zsh: `sed -i '/^qtnl()/d' ~/.zshrc
echo \\
  'qtnl(){ ssh "\${2:-user}"@t.tn3w.dev -oStrictHostKeyChecking=no -NR 80:localhost:"\${1:-3000}"; }' \\
  >> ~/.zshrc`,
    fish: `sed -i '/^function qtnl/d' ~/.config/fish/config.fish
echo \\
  'function qtnl; ssh (or $argv[2] user)@t.tn3w.dev -oStrictHostKeyChecking=no -NR 80:localhost:(or $argv[1] 3000); end' \\
  >> ~/.config/fish/config.fish`,
    csh: `sed -i '/alias qtnl/d' ~/.cshrc
echo \\
  'alias qtnl ssh \\!:2@t.tn3w.dev -oStrictHostKeyChecking=no -NR 80:localhost:\\!:1' \\
  >> ~/.cshrc`,
    pwsh: `$p=$PROFILE; (Get-Content $p 2>$null) -notmatch '^function qtnl' | Set-Content $p
Add-Content $p \\
  'function qtnl($port=3000,$sub="user"){ ssh "$sub@t.tn3w.dev" -oStrictHostKeyChecking=no -NR 80:localhost:$port }'`,
};

let activeShell = 'bash';

function renderInstallCommand() {
    const el = document.getElementById('install-command-display');
    if (el) el.textContent = INSTALL_COMMANDS[activeShell];
}

renderInstallCommand();

document.getElementById('install-tabs')?.addEventListener('click', (e) => {
    const tab = e.target.closest('.install-tab');
    if (!tab) return;
    document.querySelectorAll('.install-tab').forEach((t) => t.classList.remove('active'));
    tab.classList.add('active');
    activeShell = tab.dataset.shell;
    const label = document.getElementById('install-shell-label');
    const labelMap = {
        bash: 'bash / sh',
        zsh: 'zsh',
        fish: 'fish',
        csh: 'csh / tcsh',
        pwsh: 'powershell',
    };
    if (label) label.textContent = labelMap[activeShell] ?? activeShell;
    renderInstallCommand();
});

document.getElementById('install-copy-btn')?.addEventListener('click', () => {
    const raw = {
        bash: `sed -i '/^qtnl()/d' ~/.bashrc
echo 'qtnl(){ ssh "\${2:-user}"@t.tn3w.dev -oStrictHostKeyChecking=no -NR 80:localhost:"\${1:-3000}"; }' >> ~/.bashrc`,
        zsh: `sed -i '/^qtnl()/d' ~/.zshrc
echo 'qtnl(){ ssh "\${2:-user}"@t.tn3w.dev -oStrictHostKeyChecking=no -NR 80:localhost:"\${1:-3000}"; }' >> ~/.zshrc`,
        fish: `sed -i '/^function qtnl/d' ~/.config/fish/config.fish
echo 'function qtnl; ssh (or $argv[2] user)@t.tn3w.dev -oStrictHostKeyChecking=no -NR 80:localhost:(or $argv[1] 3000); end' >> ~/.config/fish/config.fish`,
        csh: `sed -i '/alias qtnl/d' ~/.cshrc
echo 'alias qtnl ssh \\!:2@t.tn3w.dev -oStrictHostKeyChecking=no -NR 80:localhost:\\!:1' >> ~/.cshrc`,
        pwsh: `$p=$PROFILE; (Get-Content $p 2>$null) -notmatch '^function qtnl' | Set-Content $p; Add-Content $p 'function qtnl($port=3000,$sub="user"){ ssh "$sub@t.tn3w.dev" -oStrictHostKeyChecking=no -NR 80:localhost:$port }'`,
    };
    navigator.clipboard.writeText(raw[activeShell]).catch(() => {});
    const btn = document.getElementById('install-copy-btn');
    if (!btn) return;
    btn.textContent = 'Copied!';
    btn.classList.add('copied');
    setTimeout(() => {
        btn.textContent = 'Copy';
        btn.classList.remove('copied');
    }, 2000);
});
