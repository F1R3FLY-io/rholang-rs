# Projekt Wirtualnej Maszyny Rholang Nowej Generacji

## Wprowadzenie i Wizja

Wirtualna Maszyna Rholang (VM) nowej generacji stanowi fundamentalną zmianę w sposobie wykonywania procesów współbieżnych w środowisku rozproszonym. W przeciwieństwie do tradycyjnych maszyn wirtualnych, które zarządzają współbieżnością poprzez współdzieloną pamięć i blokady, ten nowy projekt VM przyjmuje prawdziwą naturę modelu współbieżności Rholang, traktując każdy proces jako niezależną jednostkę wykonawczą z własną izolowaną maszyną stanów.

Główną wizją jest stworzenie architektury VM, w której właściwości matematyczne rachunku Rho są zachowane w całym procesie kompilacji i wykonania. Zapewnia to, że semantyka programów Rholang pozostaje spójna od kodu źródłowego do wykonania, zachowując to, co nazywamy "funktorialnością" - właściwość, że kompilacja zachowuje strukturę języka źródłowego.

Dzięki głębokiej integracji z RSpace (warstwą pamięci opartą na przestrzeni krotek), ten projekt VM umożliwia naturalne reprezentowanie procesów współbieżnych jako niezależnych jednostek, które komunikują się wyłącznie poprzez przekazywanie wiadomości. Takie podejście nie tylko upraszcza model wykonania, ale także zapewnia jasną ścieżkę do rozproszonego wykonania na wielu węzłach w sieci.

## Podstawowe Wymagania Architektoniczne

### 1. Funktorialność Cytowania

Kompilacja cytowanego procesu (@P) musi być równoważna operacji QUOTE na już skompilowanym kodzie bajtowym procesu P. Wyraża to formuła:

```
[| @P |] = QUOTE [| P |]
```

Ta zasada zapewnia, że operacja cytowania w Rholang zachowuje swoje znaczenie semantyczne w procesie kompilacji. Cytowanie jest fundamentalną operacją w rachunku Rho, która przekształca proces w nazwę, umożliwiając traktowanie procesów jako wartości pierwszej klasy, które mogą być przekazywane w wiadomościach.

Techniczne znaczenie tego wymagania jest głębokie: gwarantuje, że VM może poprawnie obsługiwać procesy wyższego rzędu (procesy, które manipulują innymi procesami). Jest to kluczowe dla implementacji zaawansowanych wzorców, takich jak kod mobilny, gdzie procesy mogą być przesyłane między różnymi częściami systemu rozproszonego i wykonywane zdalnie.

### 2. Funktorialność Współbieżności

Kompilacja równoległej kompozycji procesów (P1 | … | Pn) musi skutkować multizbiorem ({|...|}) zawierającym indywidualne skompilowane kody bajtowe każdego procesu. Wyraża to formuła:

```
[| P1 | … | Pn |] = {| [| P1 |], …, [| Pn |] |}
```

Ta zasada zapewnia, że operator kompozycji równoległej (|) w Rholang bezpośrednio przekłada się na niezależne sekwencje kodu bajtowego w skompilowanym wyniku. Reprezentacja multizbioru oddaje komutatywną naturę kompozycji równoległej - kolejność procesów nie ma znaczenia, liczy się tylko ich współbieżne wykonanie.

To wymaganie jest niezbędne dla zachowania prawdziwej semantyki współbieżności Rholang. Reprezentując procesy równoległe jako niezależne sekwencje kodu bajtowego, VM może wykonywać je naprawdę współbieżnie bez sztucznej sekwencjonalizacji, która wprowadzałaby niedeterminizm nieobecny w programie źródłowym.

### 3. Stan jako Multizbiór

Kompletny, chwilowy stan VM jest zdefiniowany jako multizbiór indywidualnych sekwencji kodu bajtowego:

```
{| [| P1 |], …, [| Pn |] |}
```

Ta zasada ustanawia, że stan VM nie jest monolitycznym bytem, ale kolekcją niezależnych stanów procesów. Każdy proces w systemie wnosi swój własny stan do ogólnego stanu VM, bez bezpośredniej interakcji ze stanami innych procesów.

To podejście do reprezentacji stanu jest kluczowe dla skalowalności i izolacji błędów. Ponieważ procesy nie współdzielą stanu bezpośrednio, awarie w jednym procesie nie uszkadzają stanu innych. Dodatkowo, ten model naturalnie wspiera dystrybucję, ponieważ różne procesy mogą być wykonywane na różnych maszynach fizycznych bez wymagania złożonych mechanizmów synchronizacji stanu.

### 4. Izolowane Wykonanie

Każda indywidualna sekwencja kodu bajtowego [| Pi |] z multizbioru musi być wykonywana w swojej własnej, oddzielnej, dedykowanej maszynie stanów. Ta maszyna stanów reprezentuje pojedynczy logiczny wątek.

Ta zasada wymusza ścisłą izolację między procesami, zapewniając, że każdy proces wykonuje się niezależnie z własnym kontekstem wykonania, stosem i zmiennymi lokalnymi. Ta izolacja jest fundamentalna dla modelu rachunku Rho, gdzie procesy wchodzą w interakcje tylko poprzez jawne kanały komunikacyjne.

Techniczne znaczenie izolowanego wykonania wykracza poza poprawność do wydajności i skalowalności. Wykonując procesy w izolowanych maszynach stanów, VM może łatwo dystrybuować wykonanie na wiele rdzeni, a nawet wiele maszyn. To podejście upraszcza również rozumowanie o zachowaniu procesu, ponieważ wykonanie każdego procesu zależy tylko od jego własnego stanu i wiadomości, które otrzymuje.

### 5. Głęboka Integracja z RSpace poprzez "Channel Trick"

Aby obsłużyć operacje, które nie są natywnymi komunikacjami Rholang (np. arytmetyka), każda maszyna stanów [| Pi |] musi mieć przypisany prywatny kanał. Ten kanał jest używany do komunikacji z "procesorem prymitywów". To osadza każdą operację, w tym operacje prymitywne, w modelu komunikacji RSpace.

Ta zasada zapewnia, że wszystkie obliczenia w VM, nawet operacje prymitywne jak arytmetyka, są wyrażane poprzez ten sam mechanizm komunikacji używany do interakcji procesów. Przypisując każdej maszynie stanów prywatny kanał, VM może kierować żądania operacji prymitywnych do wyspecjalizowanych procesorów bez łamania paradygmatu przekazywania wiadomości.

"Channel Trick" jest potężną techniką unifikacji, która upraszcza architekturę VM, redukując wszystkie obliczenia do przekazywania wiadomości. To podejście czyni VM bardziej rozszerzalną, ponieważ nowe operacje prymitywne mogą być dodawane przez proste rejestrowanie nowych procedur obsługi dla określonych wzorców wiadomości, bez modyfikowania głównego silnika wykonawczego.

### 6. Abstrakcja Wątku

"Świadkiem" dla wątku (czy to zielony wątek, czy fizyczny) jest prywatny kanał przypisany do maszyny stanów. Zarządzanie wątkami staje się procesem mapowania zasobów wykonawczych na tych reprezentantów kanałów w RSpace.

Ta zasada ustanawia jasną tożsamość dla każdego wątku wykonawczego w systemie. Używając prywatnego kanału jako tożsamości wątku, VM może śledzić, planować i zarządzać wątkami, używając tych samych mechanizmów, których używa dla innych operacji RSpace.

To podejście do abstrakcji wątku zapewnia naturalny sposób implementacji funkcji takich jak priorytetyzacja wątków, równoważenie obciążenia i alokacja zasobów. Upraszcza również implementację zaawansowanych wzorców współbieżności, takich jak rachunek join, gdzie wiele wątków synchronizuje się na współdzielonych kanałach.

## Model Implementacji i Wykonania

### Proces Kompilacji

Proces kompilacji przekształca kod źródłowy Rholang w multizbiór niezależnych sekwencji kodu bajtowego, zachowując strukturę współbieżności oryginalnego programu.

1. **Parsowanie**: Kod źródłowy Rholang jest parsowany do Abstrakcyjnego Drzewa Składniowego (AST) przy użyciu parsera wygenerowanego z gramatyki Rholang.

2. **Analiza AST**: AST jest analizowane w celu identyfikacji kompozycji równoległych, deklaracji nazw i innych konstrukcji Rholang.

3. **Generowanie Kodu Bajtowego**: Dla każdego procesu w kompozycji równoległej, kompilator generuje oddzielną sekwencję kodu bajtowego. To tutaj implementowana jest funktorialność współbieżności:

   ```
   compile(P1 | P2 | ... | Pn) = {| compile(P1), compile(P2), ..., compile(Pn) |}
   ```

4. **Optymalizacja**: Każda sekwencja kodu bajtowego jest optymalizowana niezależnie, bez wpływu na semantykę innych sekwencji.

5. **Linkowanie**: Odniesienia między procesami (np. poprzez współdzielone kanały) są rozwiązywane i odpowiednio linkowane.

Kluczową innowacją w tym procesie kompilacji jest bezpośrednie mapowanie kompozycji równoległej na niezależne sekwencje kodu bajtowego. To zachowuje strukturę współbieżności oryginalnego programu i umożliwia prawdziwie równoległe wykonanie.

### Wykonanie Maszyny Stanów

Każda sekwencja kodu bajtowego [| Pi |] jest wykonywana przez dedykowaną maszynę stanów z własnym kontekstem wykonania. Maszyna stanów utrzymuje:

1. **Wskaźnik Instrukcji**: Wskazuje na aktualnie wykonywaną instrukcję.
2. **Stos Operandów**: Przechowuje wartości pośrednie podczas obliczeń.
3. **Zmienne Lokalne**: Przechowuje zmienne zadeklarowane w procesie.
4. **Referencje Kanałów**: Utrzymuje referencje do kanałów używanych przez proces.
5. **Prywatny Kanał**: Unikalny kanał przypisany do tej maszyny stanów dla operacji prymitywnych.

Wykonanie maszyny stanów następuje w prostym cyklu:

1. Pobierz następną instrukcję z sekwencji kodu bajtowego.
2. Zdekoduj instrukcję, aby określić operację.
3. Wykonaj operację, która może:
   - Modyfikować stos operandów
   - Aktualizować zmienne lokalne
   - Wysyłać lub odbierać wiadomości na kanałach
   - Żądać operacji prymitywnych poprzez prywatny kanał
4. Zaktualizuj wskaźnik instrukcji.
5. Powtarzaj, aż sekwencja kodu bajtowego zostanie wyczerpana lub zablokowana na operacji odbioru.

Ten izolowany model wykonania upraszcza implementację VM i umożliwia prostą paralelizację. Każda maszyna stanów może być planowana niezależnie, a wiele maszyn stanów może wykonywać się współbieżnie bez złożonych mechanizmów synchronizacji.

### Interferencja RSpace i "Channel Trick"

"Channel Trick" jest kluczową innowacją, która osadza wszystkie obliczenia, w tym operacje prymitywne, w modelu komunikacji RSpace. Oto szczegółowy opis, jak maszyna stanów wykonuje operację prymitywną, taką jak `x = 5 + 3`:

1. **Dekodowanie Instrukcji**: Maszyna stanów napotyka instrukcję ADD w swojej sekwencji kodu bajtowego.

2. **Przygotowanie Operandów**: Operandy (5 i 3) są już na stosie operandów, zostały tam umieszczone przez poprzednie instrukcje.

3. **Komunikacja przez Prywatny Kanał**:
   - Maszyna stanów tworzy wiadomość zawierającą:
     - Kod operacji (ADD)
     - Operandy (5 i 3)
     - Kanał kontynuacji dla wyniku
   - Ta wiadomość jest wysyłana na prywatnym kanale maszyny stanów do procesora prymitywów.

4. **Przetwarzanie Prymitywne**:
   - Procesor prymitywów odbiera wiadomość z prywatnego kanału.
   - Wykonuje żądaną operację (5 + 3 = 8).
   - Wysyła wynik (8) na kanale kontynuacji.

5. **Odbiór Wyniku**:
   - Maszyna stanów odbiera wynik (8) z kanału kontynuacji.
   - Umieszcza wynik na stosie operandów.
   - Kontynuuje wykonanie z następną instrukcją.

To podejście ma kilka zalet:

- **Jednolitość**: Wszystkie operacje, czy to prymitywne czy wysokopoziomowe, używają tego samego mechanizmu komunikacji.
- **Rozszerzalność**: Nowe operacje prymitywne mogą być dodawane przez rejestrowanie nowych procedur obsługi w procesorze prymitywów.
- **Dystrybucja**: Operacje prymitywne mogą być wykonywane na różnych maszynach fizycznych niż żądająca maszyna stanów.
- **Izolacja Błędów**: Awarie w operacjach prymitywnych nie wpływają na stan innych procesów.

Co najważniejsze, to podejście sprawia, że kolekcja maszyn stanów jest natywnie reprezentowalna w RSpace. Każda maszyna stanów jest identyfikowana przez swój prywatny kanał, a cała interakcja z maszyną stanów odbywa się poprzez przekazywanie wiadomości na tym kanale. To umożliwia czystą integrację z warstwą pamięci RSpace i zapewnia naturalną ścieżkę do rozproszonego wykonania.

## Wywoływanie i Zarządzanie Wątkami

Pytanie "Gdzie są wywoływane wątki?" ma jasną odpowiedź w tej architekturze: wątki są wywoływane na poziomie indywidualnych maszyn stanów, każda identyfikowana przez swój prywatny kanał w RSpace.

### Tożsamość i Cykl Życia Wątku

W tej architekturze wątek nie jest tradycyjnym wątkiem OS ani nawet zielonym wątkiem w konwencjonalnym sensie. Zamiast tego, wątek jest kontekstem wykonania powiązanym z określoną maszyną stanów, identyfikowaną przez jej prywatny kanał w RSpace.

Cykl życia wątku przebiega przez następujące etapy:

1. **Tworzenie**: Gdy nowy proces jest uruchamiany (albo z początkowego programu, albo poprzez kompozycję równoległą), tworzona jest nowa maszyna stanów z własnym prywatnym kanałem.

2. **Planowanie**: Planista wybiera maszyny stanów do wykonania na podstawie dostępności zasobów i polityk planowania. Prywatny kanał służy jako uchwyt dla decyzji planowania.

3. **Wykonanie**: Wybrana maszyna stanów wykonuje swoją sekwencję kodu bajtowego, aż zakończy lub zablokuje się na operacji odbioru.

4. **Blokowanie**: Jeśli maszyna stanów blokuje się na operacji odbioru, jej stan jest zachowywany, a ona sama jest usuwana z aktywnej kolejki planowania, dopóki nie nadejdzie pasująca wiadomość.

5. **Wznowienie**: Gdy nadchodzi wiadomość, która pasuje do zablokowanego odbioru, odpowiednia maszyna stanów jest wznawiana i dodawana z powrotem do kolejki planowania.

6. **Zakończenie**: Gdy maszyna stanów kończy swoją sekwencję kodu bajtowego, jej zasoby są zwalniane, a jej prywatny kanał może być zbierany przez odśmiecacz, jeśli nie jest już referencjonowany.

### Alokacja i Planowanie Wątków

Alokacja zasobów wykonawczych (czas CPU, pamięć itp.) do wątków jest zarządzana przez system planowania, który mapuje fizyczne zasoby na maszyny stanów na podstawie ich prywatnych kanałów.

Planista utrzymuje kilka struktur danych:

1. **Aktywna Kolejka**: Zawiera prywatne kanały maszyn stanów gotowych do wykonania.
2. **Mapa Zablokowanych**: Mapuje kanały na maszyny stanów zablokowane na odbiorach.
3. **Mapa Zasobów**: Śledzi wykorzystanie zasobów przez każdą maszynę stanów.

Gdy wiadomość jest wysyłana na kanale, planista sprawdza, czy jakiekolwiek maszyny stanów są zablokowane na tym kanale. Jeśli tak, przenosi je z mapy zablokowanych do aktywnej kolejki.

Planista następnie wybiera maszyny stanów z aktywnej kolejki na podstawie polityk planowania (np. round-robin, opartych na priorytetach) i przypisuje je do dostępnych zasobów wykonawczych (np. rdzeni CPU).

To podejście do zarządzania wątkami ma kilka zalet:

- **Skalowalność**: Liczba maszyn stanów może znacznie przekraczać liczbę fizycznych rdzeni CPU, umożliwiając efektywne wykorzystanie zasobów.
- **Sprawiedliwość**: Planista może implementować różne polityki sprawiedliwości, aby zapewnić, że wszystkie procesy mają szansę na wykonanie.
- **Kontrola Zasobów**: Planista może ograniczać zasoby przydzielane każdej maszynie stanów, zapobiegając wyczerpaniu zasobów.
- **Równoważenie Obciążenia**: W środowisku rozproszonym, maszyny stanów mogą być migrowane między fizycznymi węzłami w celu równoważenia obciążenia.

### Wykonanie Rozproszone

Model wątków opisany powyżej naturalnie rozszerza się na wykonanie rozproszone na wielu maszynach fizycznych. Ponieważ każda maszyna stanów jest identyfikowana przez swój prywatny kanał, a cała interakcja odbywa się poprzez przekazywanie wiadomości, fizyczna lokalizacja maszyny stanów jest przezroczysta dla innych procesów.

W środowisku rozproszonym:

1. **Routing Kanałów**: Wiadomości wysyłane na kanałach są kierowane do odpowiedniego węzła fizycznego na podstawie lokalizacji odbierającej maszyny stanów.

2. **Migracja Maszyn Stanów**: Maszyny stanów mogą być migrowane między węzłami w celu równoważenia obciążenia lub tolerancji błędów.

3. **Rozproszone Planowanie**: Rozproszony planista koordynuje alokację zasobów na wielu węzłach.

4. **Tolerancja Błędów**: Jeśli węzeł ulegnie awarii, maszyny stanów działające na tym węźle mogą być odzyskane z trwałej pamięci i wznowione na innych węzłach.

Ten rozproszony model wykonania idealnie pasuje do warstwy pamięci RSpace, która już zapewnia mechanizmy do rozproszonego przechowywania i pobierania krotek. Reprezentując maszyny stanów jako jednostki w RSpace, VM może wykorzystać te mechanizmy do rozproszonego wykonania.

## Podsumowanie

Projekt VM Rholang nowej generacji przedstawiony w tym dokumencie reprezentuje znaczący postęp w wykonywaniu programów współbieżnych, rozproszonych. Przestrzegając sześciu podstawowych zasad - funktorialności cytowania, funktorialności współbieżności, stanu jako multizbioru, izolowanego wykonania, głębokiej integracji z RSpace i abstrakcji wątku - ten projekt tworzy VM, która prawdziwie ucieleśnia matematyczne podstawy rachunku Rho.

Kluczowe innowacje w tym projekcie opartym na ścieżkach obejmują:

1. Bezpośrednią reprezentację kompozycji równoległej jako niezależnych ścieżek w strukturze hierarchicznej.
2. Wykonanie każdego procesu w jego własnym izolowanym kontekście ścieżki.
3. Wykorzystanie kanałów świadomych ścieżek do osadzenia wszystkich obliczeń w modelu komunikacji RSpace.
4. Identyfikację wątków z kontekstami ścieżek w RSpace.
5. Hierarchiczną organizację ścieżek, która odzwierciedla strukturę współbieżności programu.
6. Efektywne mechanizmy synchronizacji oparte na relacjach między ścieżkami.

Te innowacje umożliwiają VM, która jest nie tylko matematycznie spójna, ale także wysoce skalowalna, odporna na błędy i naturalnie rozproszona. Architektura oparta na ścieżkach idealnie pasuje do struktury AST programów Rholang, czyniąc proces kompilacji bardziej przejrzystym i zachowując właściwości semantyczne języka źródłowego.

Budując na solidnych podstawach rachunku Rho, integrując się głęboko z RSpace i wykorzystując model wykonania oparty na ścieżkach, ten projekt VM zapewnia potężną platformę dla następnej generacji współbieżnych, rozproszonych aplikacji.