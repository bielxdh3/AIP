# Productization human smoke

Status: HUMAN VALIDATION PENDING. Esta lista é o último gate de pacote/Windows; não substitui os testes automatizados.

1. Abra o MSI/NSIS instalado e confirme que a superfície **Capacidades locais** está visível sem abrir painéis ocultos.
2. Em **Voz**, selecione microfone/saída reais e, em **Registro de provedores locais**, informe um executável absoluto, protocolo correto e use **Validar e registrar**. Confirme estado `ready`, selecione-o como STT/TTS e execute uma amostra curta. Sem provedor, confirme degradação explícita para texto.
3. Em **Ferramentas**, adicione uma raiz de teste pela UI. Execute inspeção, prévia, dry-run, aprovação, segunda confirmação, mover, rollback e tente um caminho fora da raiz; confirme bloqueio e auditoria.
4. Em **Extensões**, importe a fixture declarativa, revise capacidades, ative, execute, atualize (revisão obrigatória), faça rollback e disable. Confirme que não há shell, rede, credenciais ou arquivos arbitrários.
5. Em **Visão de tela**, confirme que displays reais são listados, capture somente após confirmação, verifique redaction/metadata e execute com provedor registrado ou confirme estado degradado sem sucesso sintético.
6. Em **Gateway**, inicie o listener local, confirme endpoint loopback e pairing transitório, verifique parada e que nenhuma conta/transferência externa é afirmada.
7. Em **Companion Android**, inicie/parar o transporte de depuração loopback e, opcionalmente, conecte um APK debug em dispositivo autorizado. Confirme que o caminho é explícito e não é relay de produção.
8. Navegue por 7B–7F (opiniões, relações, metas/atividades, conversa pública e validação) e confirme que cada fluxo é descobrível, persiste após reinício e mantém os avisos de conteúdo simulado.

Registre versão do instalador, SHA-256, sistema Windows, dispositivos usados e qualquer falha. Não cole caminhos privados, tokens, pairing codes ou conteúdo de tela no relatório.
