; Instalador de nook (Inno Setup 6). Lo compila el workflow de las releases
; (.github/workflows/release.yml) con la versión del tag:
;   ISCC /DVersion=0.3.0 instalador\nook.iss
; Necesita antes `cargo build --release` y deja target\instalador\nook-setup.exe.
;
; Se instala solo para el usuario, sin pedir permisos de administrador, en
; %LOCALAPPDATA%\Programs\nook: así la app puede actualizarse sola cambiando su propio
; ejecutable (ver src/actualizacion.rs).

#ifndef Version
  #define Version "0.0.0"
#endif
#define Exe "nook.exe"
; Inno Setup trae el coreano solo en sus versiones más nuevas.
#define HayCoreano FileExists(AddBackslash(CompilerPath) + "Languages\Korean.isl")

[Setup]
AppId={{7AA6FA16-B489-45C4-A8E4-E0F804C678B3}
AppName=Nookly
AppVersion={#Version}
AppVerName=Nookly {#Version}
AppPublisher=fate-9x
AppPublisherURL=https://github.com/fate-9x/nook
AppSupportURL=https://github.com/fate-9x/nook/issues
AppUpdatesURL=https://github.com/fate-9x/nook/releases
VersionInfoVersion={#Version}
PrivilegesRequired=lowest
DefaultDirName={autopf}\nook
; La carpeta se puede elegir siempre, también al actualizar (ver NextButtonClick y
; CurStepChanged, más abajo).
DisableDirPage=no
DisableProgramGroupPage=yes
DefaultGroupName=Nookly
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
WizardStyle=modern
SetupIconFile=..\assets\nook.ico
; Imágenes del asistente con el personaje (la grande en la primera y la última pantalla, la
; pequeña arriba a la derecha), en cada escala de pantalla. Se generan con
;   cargo test genera_las_imagenes_del_instalador -- --ignored
WizardImageFile=..\assets\instalador\grande-164.bmp,..\assets\instalador\grande-205.bmp,..\assets\instalador\grande-246.bmp,..\assets\instalador\grande-287.bmp,..\assets\instalador\grande-328.bmp,..\assets\instalador\grande-410.bmp
WizardSmallImageFile=..\assets\instalador\pequena-58.bmp,..\assets\instalador\pequena-73.bmp,..\assets\instalador\pequena-87.bmp,..\assets\instalador\pequena-102.bmp,..\assets\instalador\pequena-116.bmp,..\assets\instalador\pequena-145.bmp
UninstallDisplayIcon={app}\{#Exe}
UninstallDisplayName=Nookly
OutputDir=..\target\instalador
OutputBaseFilename=nook-setup
Compression=lzma2/max
SolidCompression=yes
; La app se cierra desde el código de abajo (solo la copia instalada, no otras).
CloseApplications=no
ShowLanguageDialog=auto
LanguageDetectionMethod=uilanguage

[Languages]
Name: "es"; MessagesFile: "compiler:Languages\Spanish.isl"
Name: "en"; MessagesFile: "compiler:Default.isl"
Name: "ja"; MessagesFile: "compiler:Languages\Japanese.isl"
#if HayCoreano
Name: "ko"; MessagesFile: "compiler:Languages\Korean.isl"
#endif

[CustomMessages]
es.IniciarConWindows=Iniciar Nookly con Windows
es.Opciones=Opciones:
es.CarpetaSinPermiso=No se puede escribir en esta carpeta sin permisos de administrador.%n%nNookly necesita una carpeta en la que pueda escribir para actualizarse solo. Elige otra, por ejemplo dentro de tu carpeta de usuario.
es.BorrarDatos=¿Borrar también tus ajustes, grupos y textos guardados de Nookly?%n%nSi vas a volver a instalarlo, elige No.
en.IniciarConWindows=Start Nookly with Windows
en.Opciones=Options:
en.CarpetaSinPermiso=This folder can't be written to without administrator permissions.%n%nNookly needs a folder it can write to so it can update itself. Choose another one, for example inside your user folder.
en.BorrarDatos=Also delete your Nookly settings, groups and saved texts?%n%nIf you are going to install it again, choose No.
ja.IniciarConWindows=Windows の起動時に Nookly を開始する
ja.Opciones=オプション:
ja.CarpetaSinPermiso=このフォルダーには管理者権限なしでは書き込めません。%n%nNookly が自動で更新するには、書き込めるフォルダーが必要です。ユーザー フォルダー内など、別のフォルダーを選んでください。
ja.BorrarDatos=Nookly の設定、グループ、保存したテキストも削除しますか?%n%n再インストールする場合は「いいえ」を選んでください。
#if HayCoreano
ko.IniciarConWindows=Windows 시작 시 Nookly 실행
ko.Opciones=옵션:
ko.CarpetaSinPermiso=관리자 권한 없이는 이 폴더에 쓸 수 없습니다.%n%nNookly가 스스로 업데이트하려면 쓸 수 있는 폴더가 필요합니다. 사용자 폴더 안 등 다른 폴더를 선택하세요.
ko.BorrarDatos=Nookly의 설정, 그룹, 저장한 텍스트도 삭제할까요?%n%n다시 설치할 예정이라면 [아니요]를 선택하세요.
#endif

[Tasks]
Name: "inicio"; Description: "{cm:IniciarConWindows}"; GroupDescription: "{cm:Opciones}"
Name: "escritorio"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "..\target\release\{#Exe}"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Nookly"; Filename: "{app}\{#Exe}"
Name: "{autodesktop}\Nookly"; Filename: "{app}\{#Exe}"; Tasks: escritorio

[Registry]
; El mismo valor que pone el interruptor "Iniciar con Windows" de la app (src/sistema.rs). Al
; desinstalar se quita también si se activó desde la app.
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: string; ValueName: "Nook"; ValueData: """{app}\{#Exe}"""; Tasks: inicio
Root: HKCU; Subkey: "Software\Microsoft\Windows\CurrentVersion\Run"; ValueType: none; ValueName: "Nook"; Flags: uninsdeletevalue dontcreatekey

[UninstallDelete]
; Lo que deja la app al actualizarse sola.
Type: files; Name: "{app}\{#Exe}.viejo"
Type: files; Name: "{app}\{#Exe}.nuevo"

[Run]
Filename: "{app}\{#Exe}"; Description: "{cm:LaunchProgram,Nookly}"; Flags: nowait postinstall skipifsilent

[Code]
const
  // Donde Inno Setup apunta la instalación anterior (con PrivilegesRequired=lowest, en HKCU).
  ClaveDesinstalar = 'Software\Microsoft\Windows\CurrentVersion\Uninstall\{7AA6FA16-B489-45C4-A8E4-E0F804C678B3}_is1';
  ClaveInicio = 'Software\Microsoft\Windows\CurrentVersion\Run';

var
  // Carpeta de la instalación anterior ('' si no hay).
  CarpetaAnterior: String;

// Cierra la copia de Nook de esa carpeta (la que se va a cambiar o quitar) si está abierta.
procedure CerrarNookEn(Carpeta: String);
var
  Codigo: Integer;
begin
  Exec(ExpandConstant('{sys}\WindowsPowerShell\v1.0\powershell.exe'),
    '-NoProfile -NonInteractive -Command "Get-Process nook -ErrorAction SilentlyContinue | ' +
    'Where-Object { $_.Path -eq ''' + AddBackslash(Carpeta) + '{#Exe}' + ''' } | Stop-Process -Force"',
    '', SW_HIDE, ewWaitUntilTerminated, Codigo);
  // Que Windows suelte el ejecutable antes de cambiarlo o borrarlo.
  Sleep(500);
end;

procedure CerrarNook;
begin
  CerrarNookEn(ExpandConstant('{app}'));
end;

function MismaCarpeta(A, B: String): Boolean;
begin
  Result := CompareText(RemoveBackslashUnlessRoot(A), RemoveBackslashUnlessRoot(B)) = 0;
end;

function InitializeSetup: Boolean;
begin
  if not RegQueryStringValue(HKCU, ClaveDesinstalar, 'Inno Setup: App Path', CarpetaAnterior) then
    CarpetaAnterior := '';
  Result := True;
end;

// Nook se actualiza solo cambiando su propio ejecutable: la carpeta elegida tiene que poder
// escribirse sin permisos de administrador (no vale, por ejemplo, Archivos de programa).
function NextButtonClick(CurPageID: Integer): Boolean;
var
  Carpeta, Prueba: String;
  Existia, Escribible: Boolean;
begin
  Result := True;
  if CurPageID <> wpSelectDir then
    exit;
  Carpeta := WizardDirValue;
  Prueba := AddBackslash(Carpeta) + 'nook-prueba-de-escritura.tmp';
  Existia := DirExists(Carpeta);
  Escribible := ForceDirectories(Carpeta) and SaveStringToFile(Prueba, '', False);
  DeleteFile(Prueba);
  if not Existia then
    RemoveDir(Carpeta);
  if not Escribible then
  begin
    MsgBox(CustomMessage('CarpetaSinPermiso'), mbError, MB_OK);
    Result := False;
  end;
end;

function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  CerrarNook;
  if (CarpetaAnterior <> '') and not MismaCarpeta(CarpetaAnterior, ExpandConstant('{app}')) then
    CerrarNookEn(CarpetaAnterior);
  Result := '';
end;

// Si se cambió de carpeta al actualizar, se quita la copia anterior y, si Nook se inicia con
// Windows, se apunta a la nueva.
procedure CurStepChanged(CurStep: TSetupStep);
var
  Inicio: String;
begin
  if (CurStep <> ssPostInstall) or (CarpetaAnterior = '') or
     MismaCarpeta(CarpetaAnterior, ExpandConstant('{app}')) then
    exit;
  DeleteFile(AddBackslash(CarpetaAnterior) + '{#Exe}');
  DeleteFile(AddBackslash(CarpetaAnterior) + '{#Exe}.viejo');
  DeleteFile(AddBackslash(CarpetaAnterior) + '{#Exe}.nuevo');
  DeleteFile(AddBackslash(CarpetaAnterior) + 'unins000.exe');
  DeleteFile(AddBackslash(CarpetaAnterior) + 'unins000.dat');
  RemoveDir(CarpetaAnterior);
  if RegQueryStringValue(HKCU, ClaveInicio, 'Nook', Inicio) then
    RegWriteStringValue(HKCU, ClaveInicio, 'Nook', '"' + ExpandConstant('{app}\{#Exe}') + '"');
end;

function InitializeUninstall: Boolean;
begin
  CerrarNook;
  Result := True;
end;

// Los ajustes y la bóveda (%APPDATA%\nook) se conservan salvo que el usuario diga lo contrario.
procedure CurUninstallStepChanged(CurUninstallStep: TUninstallStep);
var
  Datos: String;
  Codigo: Integer;
begin
  // Si se preparó la tecla Windows con permisos de administrador (src/tecla_windows.rs), se
  // quitan su tarea programada y su ayudante; Windows pide permiso para ello.
  if (CurUninstallStep = usUninstall) and
     FileExists(ExpandConstant('{commonpf64}\Nook Ayudante\nook-tecla-windows.exe')) then
    ShellExec('runas', ExpandConstant('{app}\{#Exe}'), '--quitar-tecla-windows', '', SW_HIDE,
      ewWaitUntilTerminated, Codigo);
  Datos := ExpandConstant('{userappdata}\nook');
  if (CurUninstallStep = usPostUninstall) and DirExists(Datos) and not UninstallSilent then
    if MsgBox(CustomMessage('BorrarDatos'), mbConfirmation, MB_YESNO or MB_DEFBUTTON2) = IDYES then
      DelTree(Datos, True, True, True);
end;
