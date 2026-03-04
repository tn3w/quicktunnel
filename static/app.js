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
    bash: `grep -qxF 'qtnl(){ ssh -oStrictHostKeyChecking=no -NR 80:localhost:"$1" t.tn3w.dev; }' \\
  ~/.bashrc \\
  || echo 'qtnl(){ ssh -oStrictHostKeyChecking=no -NR 80:localhost:"$1" t.tn3w.dev; }' \\
  >> ~/.bashrc`,
    zsh: `grep -qxF 'qtnl(){ ssh -oStrictHostKeyChecking=no -NR 80:localhost:"$1" t.tn3w.dev; }' \\
  ~/.zshrc \\
  || echo 'qtnl(){ ssh -oStrictHostKeyChecking=no -NR 80:localhost:"$1" t.tn3w.dev; }' \\
  >> ~/.zshrc`,
    fish: `grep -qx 'function qtnl; ssh -oStrictHostKeyChecking=no -NR 80:localhost:$argv[1] t.tn3w.dev; end' \\
  ~/.config/fish/config.fish \\
  || echo 'function qtnl; ssh -oStrictHostKeyChecking=no -NR 80:localhost:$argv[1] t.tn3w.dev; end' \\
  >> ~/.config/fish/config.fish`,
    csh: `grep -q 'alias qtnl' ~/.cshrc \\
  || echo 'alias qtnl ssh -oStrictHostKeyChecking=no -NR 80:localhost:\\!^ t.tn3w.dev' \\
  >> ~/.cshrc`,
    pwsh: `if(!(Select-String -Quiet 'qtnl' $PROFILE 2>$null)){
  Add-Content $PROFILE \`
    'function qtnl($p){ ssh -oStrictHostKeyChecking=no -NR 80:localhost:$p t.tn3w.dev }'
}`,
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
        bash: `grep -qxF 'qtnl(){ ssh -oStrictHostKeyChecking=no -NR 80:localhost:"$1" t.tn3w.dev; }' ~/.bashrc || echo 'qtnl(){ ssh -oStrictHostKeyChecking=no -NR 80:localhost:"$1" t.tn3w.dev; }' >> ~/.bashrc`,
        zsh: `grep -qxF 'qtnl(){ ssh -oStrictHostKeyChecking=no -NR 80:localhost:"$1" t.tn3w.dev; }' ~/.zshrc || echo 'qtnl(){ ssh -oStrictHostKeyChecking=no -NR 80:localhost:"$1" t.tn3w.dev; }' >> ~/.zshrc`,
        fish: `grep -qx 'function qtnl; ssh -oStrictHostKeyChecking=no -NR 80:localhost:$argv[1] t.tn3w.dev; end' ~/.config/fish/config.fish || echo 'function qtnl; ssh -oStrictHostKeyChecking=no -NR 80:localhost:$argv[1] t.tn3w.dev; end' >> ~/.config/fish/config.fish`,
        csh: `grep -q 'alias qtnl' ~/.cshrc || echo 'alias qtnl ssh -oStrictHostKeyChecking=no -NR 80:localhost:\\!^ t.tn3w.dev' >> ~/.cshrc`,
        pwsh: `if(!(Select-String -Quiet 'qtnl' $PROFILE 2>$null)){ Add-Content $PROFILE 'function qtnl($p){ ssh -oStrictHostKeyChecking=no -NR 80:localhost:$p t.tn3w.dev }' }`,
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

const SHARE_COMMANDS = {
    linux: {
        python: `python3 -m http.server 8080 & ssh -oStrictHostKeyChecking=no -NR 80:localhost:8080 t.tn3w.dev`,
        node: `npx serve . -l 8080 & ssh -oStrictHostKeyChecking=no -NR 80:localhost:8080 t.tn3w.dev`,
        native: `(p=8080; while true; do { echo -e "HTTP/1.1 200 OK\\r\\nContent-Type: text/html\\r\\n\\r\\n"; cat index.html; } | nc -l -q1 $p; done) & ssh -oStrictHostKeyChecking=no -NR 80:localhost:8080 t.tn3w.dev`,
    },
    macos: {
        python: `python3 -m http.server 8080 & ssh -oStrictHostKeyChecking=no -NR 80:localhost:8080 t.tn3w.dev`,
        node: `npx serve . -l 8080 & ssh -oStrictHostKeyChecking=no -NR 80:localhost:8080 t.tn3w.dev`,
        native: `ruby -run -e httpd . -p 8080 & ssh -oStrictHostKeyChecking=no -NR 80:localhost:8080 t.tn3w.dev`,
    },
    windows: {
        python: `Start-Process python3 -ArgumentList "-m", "http.server", "8080" -NoNewWindow; ssh -oStrictHostKeyChecking=no -NR 80:localhost:8080 t.tn3w.dev`,
        node: `Start-Process npx -ArgumentList "serve", ".", "-l", "8080" -NoNewWindow; ssh -oStrictHostKeyChecking=no -NR 80:localhost:8080 t.tn3w.dev`,
        native: `Start-Job -ScriptBlock { $p=8080; $l=[Net.HttpListener]::new(); $l.Prefixes.Add("http://+:$p/"); $l.Start(); while($true){$c=$l.GetContext(); $f=Join-Path $pwd $c.Request.Url.LocalPath.TrimStart('/'); $b=if(Test-Path $f){[IO.File]::ReadAllBytes($f)}else{$c.Response.StatusCode=404;@()}; $c.Response.OutputStream.Write($b,0,$b.Length); $c.Response.Close()} }; ssh -oStrictHostKeyChecking=no -NR 80:localhost:8080 t.tn3w.dev`,
    },
};

const SHARE_SHELL_LABELS = {
    linux: 'bash / zsh / sh',
    macos: 'bash / zsh / sh',
    windows: 'powershell',
};

const NATIVE_LABELS = {
    linux: 'Pure Bash',
    macos: 'Ruby',
    windows: 'Pure PowerShell',
};

function detectOS() {
    const ua = navigator.userAgent.toLowerCase();
    if (ua.includes('win')) return 'windows';
    if (ua.includes('mac')) return 'macos';
    return 'linux';
}

let activeShareOS = detectOS();
let activeShareVariant = 'python';

function renderShareSection() {
    const cmds = SHARE_COMMANDS[activeShareOS];
    const shellLbl = SHARE_SHELL_LABELS[activeShareOS];

    const ids = { python: 'share-cmd-python', node: 'share-cmd-node', native: 'share-cmd-native' };
    Object.entries(ids).forEach(([key, id]) => {
        const el = document.getElementById(id);
        if (el) el.textContent = cmds[key];
    });

    document.querySelectorAll('#share .share-shell-label').forEach((el) => {
        el.textContent = shellLbl;
    });

    const nativeLabelEl = document.querySelector(
        '.share-vtab[data-variant="native"] .native-label'
    );
    if (nativeLabelEl) nativeLabelEl.textContent = NATIVE_LABELS[activeShareOS];
}

function activateShareVariant(variant) {
    activeShareVariant = variant;
    document.querySelectorAll('.share-variant').forEach((el) => {
        el.classList.toggle('active', el.dataset.variant === variant);
    });
    document.querySelectorAll('.share-vtab').forEach((btn) => {
        btn.classList.toggle('active', btn.dataset.variant === variant);
    });
}

function activateShareOS(os) {
    activeShareOS = os;
    document.querySelectorAll('.os-tab').forEach((btn) => {
        const isActive = btn.dataset.os === os;
        btn.classList.toggle('active', isActive);
        btn.classList.remove('autodetected');
    });
    renderShareSection();
}

(function initShare() {
    renderShareSection();
    activateShareOS(activeShareOS);
    activateShareVariant('python');

    const detectedLabel = document.getElementById('os-detected');
    if (detectedLabel) detectedLabel.classList.add('visible');
})();

document.getElementById('os-tabs')?.addEventListener('click', (e) => {
    const tab = e.target.closest('.os-tab');
    if (!tab) return;
    const detectedLabel = document.getElementById('os-detected');
    if (detectedLabel) detectedLabel.classList.remove('visible');
    activateShareOS(tab.dataset.os);
});

document.getElementById('share-variant-tabs')?.addEventListener('click', (e) => {
    const tab = e.target.closest('.share-vtab');
    if (!tab) return;
    activateShareVariant(tab.dataset.variant);
});

document.querySelectorAll('.share-copy-btn').forEach((btn) => {
    btn.addEventListener('click', () => {
        const variant = btn.closest('.share-variant')?.dataset.variant ?? activeShareVariant;
        const text = SHARE_COMMANDS[activeShareOS][variant] ?? '';
        navigator.clipboard.writeText(text).catch(() => {});
        btn.textContent = 'Copied!';
        btn.classList.add('copied');
        setTimeout(() => {
            btn.textContent = 'Copy';
            btn.classList.remove('copied');
        }, 2000);
    });
});
