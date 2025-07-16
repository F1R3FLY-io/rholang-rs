# Projekt Wirtualnej Maszyny Rholang Nowej Generacji

## Wprowadzenie i Wizja

Wirtualna Maszyna Rholang (VM) nowej generacji stanowi fundamentalną zmianę w sposobie wykonywania procesów współbieżnych w środowisku rozproszonym. W przeciwieństwie do tradycyjnych maszyn wirtualnych, które zarządzają współbieżnością poprzez współdzieloną pamięć i blokady, ten nowy projekt VM przyjmuje prawdziwą naturę modelu współbieżności Rholang, traktując każdy proces jako niezależną jednostkę wykonawczą z własną izolowaną maszyną stanów.

Główną wizją jest stworzenie architektury VM, w której właściwości matematyczne rachunku Rho są zachowane w całym procesie kompilacji i wykonania. Zapewnia to, że semantyka programów Rholang pozostaje spójna od kodu źródłowego do wykonania, zachowując to, co nazywamy "funktorialnością" - właściwość, że kompilacja zachowuje strukturę języka źródłowego.

Dzięki głębokiej integracji z RSpace (warstwą pamięci opartą na przestrzeni krotek), ten projekt VM umożliwia naturalne reprezentowanie procesów współbieżnych jako niezależnych jednostek, które komunikują się wyłącznie poprzez przekazywanie wiadomości. Takie podejście nie tylko upraszcza model wykonania, ale także zapewnia jasną ścieżkę do rozproszonego wykonania na wielu węzłach w sieci.

Ten dokument projektowy opiera się na specyfikacjach kodu bajtowego przedstawionych w naszej dokumentacji technicznej, szczególnie na architekturze kodu bajtowego opartej na ścieżkach, która idealnie pasuje do matematycznych podstaw rachunku Rho. Projekt kodu bajtowego dostarcza szczegółów implementacyjnych niskiego poziomu, które umożliwiają tę architekturę VM, z konkretnymi instrukcjami do obsługi kompozycji równoległej, tworzenia nazw, komunikacji i innych podstawowych konstrukcji Rholang.

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

1. **Analiza Leksykalna i Parsowanie**: Kod źródłowy Rholang jest tokenizowany i parsowany przy użyciu parsera Tree-Sitter, który produkuje konkretne drzewo składniowe (CST).

2. **Konstrukcja AST**: CST jest konwertowane do Abstrakcyjnego Drzewa Składniowego (AST) przy użyciu ASTBuilder. AST reprezentuje strukturę programu z węzłami dla różnych konstrukcji Rholang:
   - Literały (Nil, Bool, Long, String, Uri)
   - Kolekcje (List, Tuple, Set, Map)
   - Konstrukcje procesów (Par, IfThenElse, Send, ForComprehension, Match, itp.)
   - Wyrażenia (Eval, Quote, Method, UnaryExp, BinaryExp)

3. **Generowanie Kodu Bajtowego Opartego na Ścieżkach**: AST jest przekształcane w kod bajtowy przy użyciu podejścia opartego na ścieżkach. Każdemu procesowi przypisywana jest ścieżka wykonania, a kompozycje równoległe rozgałęziają się na wiele ścieżek:

   ```
   compile(P1 | P2 | ... | Pn) = {| compile(P1), compile(P2), ..., compile(Pn) |}
   ```

   Podejście oparte na ścieżkach jawnie reprezentuje konteksty wykonania i ich relacje, umożliwiając:
   - Izolowane wykonanie procesów
   - Właściwe zakresowanie i wiązanie zmiennych
   - Efektywną komunikację między procesami
   - Jasną reprezentację współbieżności

4. **Optymalizacja**: Każda sekwencja kodu bajtowego jest optymalizowana niezależnie, bez wpływu na semantykę innych sekwencji.

5. **Linkowanie**: Odniesienia między procesami (np. poprzez współdzielone kanały) są rozwiązywane i odpowiednio linkowane.

Kluczową innowacją w tym procesie kompilacji jest reprezentacja oparta na ścieżkach dla procesów współbieżnych. To zachowuje strukturę współbieżności oryginalnego programu i umożliwia prawdziwie równoległe wykonanie przy zachowaniu właściwej izolacji i kanałów komunikacyjnych.

### Wykonanie Maszyny Stanów Opartej na Ścieżkach

Każda sekwencja kodu bajtowego [| Pi |] jest wykonywana przez dedykowaną maszynę stanów w ramach własnej ścieżki wykonania. Maszyna stanów oparta na ścieżkach utrzymuje:

1. **Wskaźnik Instrukcji**: Wskazuje na aktualnie wykonywaną instrukcję.
2. **Stos Operandów**: Przechowuje wartości pośrednie podczas obliczeń.
3. **Kontekst Ścieżki**: Zawiera środowisko wykonania specyficzne dla tej ścieżki.
4. **Zmienne Lokalne**: Przechowuje zmienne zadeklarowane w procesie, powiązane ze ścieżką.
5. **Referencje Kanałów**: Utrzymuje referencje do kanałów używanych przez proces.
6. **Prywatny Kanał**: Unikalny kanał przypisany do tej ścieżki dla operacji prymitywnych.
7. **Relacje Ścieżek**: Referencje do ścieżek nadrzędnych, podrzędnych i równorzędnych.

Wykonanie maszyny stanów opartej na ścieżkach następuje w tym cyklu:

1. Pobierz następną instrukcję z sekwencji kodu bajtowego.
2. Zdekoduj instrukcję, aby określić operację.
3. Wykonaj operację, która może:
   - Modyfikować stos operandów
   - Aktualizować zmienne lokalne w kontekście ścieżki
   - Rozgałęziać nowe ścieżki dla wykonania współbieżnego
   - Łączyć się z innymi ścieżkami w punktach synchronizacji
   - Wysyłać lub odbierać wiadomości na kanałach między ścieżkami
   - Żądać operacji prymitywnych poprzez prywatny kanał
4. Zaktualizuj wskaźnik instrukcji i stan ścieżki.
5. Powtarzaj, aż sekwencja kodu bajtowego zostanie wyczerpana lub zablokowana na operacji odbioru.

Ten model wykonania oparty na ścieżkach zapewnia kilka zalet:

- **Jawna Współbieżność**: Ścieżki bezpośrednio reprezentują współbieżne konteksty wykonania.
- **Wyraźna Izolacja**: Każda ścieżka ma swój własny izolowany stan.
- **Ustrukturyzowana Komunikacja**: Ścieżki komunikują się poprzez dobrze zdefiniowane kanały.
- **Organizacja Hierarchiczna**: Ścieżki tworzą strukturę drzewiastą, która odzwierciedla współbieżność programu.
- **Efektywna Synchronizacja**: Ścieżki mogą synchronizować się na barierach bez złożonych mechanizmów blokowania.

Podejście oparte na ścieżkach idealnie pasuje do struktury AST, czyniąc proces kompilacji bardziej przejrzystym i zachowując właściwości semantyczne programu źródłowego.

### Integracja z RSpace Oparta na Ścieżkach i "Channel Trick"

"Channel Trick" jest wzmocniony w architekturze opartej na ścieżkach, aby osadzić wszystkie obliczenia, w tym operacje prymitywne, w modelu komunikacji RSpace. Oto szczegółowy opis, jak maszyna stanów oparta na ścieżkach wykonuje operację prymitywną, taką jak `x = 5 + 3`:

1. **Dekodowanie Instrukcji**: Maszyna stanów napotyka instrukcję ADD w swojej sekwencji kodu bajtowego.

2. **Przygotowanie Operandów**: Operandy (5 i 3) są ewaluowane w bieżącym kontekście ścieżki i umieszczane na stosie operandów.

3. **Komunikacja Świadoma Ścieżek**:
   - Maszyna stanów tworzy wiadomość zawierającą:
     - Kod operacji (ADD)
     - Operandy (5 i 3)
     - Identyfikator kontekstu ścieżki
     - Kanał kontynuacji dla wyniku
   - Ta wiadomość jest wysyłana na prywatnym kanale maszyny stanów do procesora prymitywów.

4. **Przetwarzanie Prymitywne Świadome Ścieżek**:
   - Procesor prymitywów odbiera wiadomość i identyfikuje kontekst ścieżki.
   - Wykonuje żądaną operację (5 + 3 = 8) w kontekście określonej ścieżki.
   - Wysyła wynik (8) na kanale kontynuacji, oznaczony identyfikatorem ścieżki.

5. **Odbiór Wyniku Świadomy Ścieżek**:
   - Maszyna stanów odbiera wynik (8) z kanału kontynuacji.
   - Weryfikuje kontekst ścieżki i aktualizuje stan ścieżki.
   - Umieszcza wynik na stosie operandów.
   - Kontynuuje wykonanie z następną instrukcją.

To podejście oparte na ścieżkach wzmacnia "Channel Trick" kilkoma zaletami:

- **Świadomość Kontekstu Ścieżki**: Operacje są wykonywane w kontekście określonych ścieżek, zachowując izolację.
- **Komunikacja Hierarchiczna**: Wiadomości mogą być kierowane przez hierarchię ścieżek, odzwierciedlając strukturę programu.
- **Efektywna Synchronizacja Ścieżek**: Wiele ścieżek może synchronizować się na barierach przy użyciu prymitywów RSpace.
- **Zarządzanie Zasobami Oparte na Ścieżkach**: Zasoby mogą być alokowane i zwalniane na podstawie cykli życia ścieżek.
- **Migracja Ścieżek**: Całe ścieżki mogą być migrowane między węzłami dla równoważenia obciążenia lub tolerancji błędów.

Architektura oparta na ścieżkach czyni integrację z RSpace jeszcze bardziej naturalną. Każda ścieżka jest identyfikowana przez swój kontekst w RSpace, a cała interakcja ze ścieżką odbywa się poprzez przekazywanie wiadomości na kanałach powiązanych z tym kontekstem. To umożliwia czystą integrację z warstwą pamięci RSpace i zapewnia potężny model dla wykonania rozproszonego.

## Zarządzanie Wątkami Oparte na Ścieżkach

Pytanie "Gdzie są wywoływane wątki?" ma jasną odpowiedź w tej architekturze opartej na ścieżkach: wątki są wywoływane na poziomie ścieżek wykonania, każda identyfikowana przez swój kontekst ścieżki w RSpace.

### Tożsamość i Cykl Życia Ścieżki

W tej architekturze wątek nie jest tradycyjnym wątkiem OS ani nawet zielonym wątkiem w konwencjonalnym sensie. Zamiast tego, wątek jest kontekstem wykonania powiązanym z określoną ścieżką, identyfikowaną przez jej kontekst ścieżki w RSpace.

Cykl życia wątku opartego na ścieżce przebiega przez następujące etapy:

1. **Tworzenie Ścieżki**: Gdy nowy proces jest uruchamiany (albo z początkowego programu, albo poprzez kompozycję równoległą), tworzona jest nowa ścieżka z własnym kontekstem.

2. **Rozgałęzianie Ścieżki**: Kompozycje równoległe rozgałęziają bieżącą ścieżkę na wiele ścieżek potomnych, z których każda wykonuje oddzielny proces.

3. **Planowanie Ścieżki**: Planista wybiera ścieżki do wykonania na podstawie dostępności zasobów i polityk planowania. Kontekst ścieżki służy jako uchwyt dla decyzji planowania.

4. **Wykonanie Ścieżki**: Wybrana ścieżka wykonuje swoją sekwencję kodu bajtowego, aż zakończy, zablokuje się na operacji odbioru lub osiągnie punkt synchronizacji.

5. **Blokowanie Ścieżki**: Jeśli ścieżka blokuje się na operacji odbioru, jej stan jest zachowywany, a ona sama jest usuwana z aktywnej kolejki planowania, dopóki nie nadejdzie pasująca wiadomość.

6. **Synchronizacja Ścieżek**: Ścieżki mogą synchronizować się na barierach, czekając na inne ścieżki, aby osiągnęły określone punkty przed kontynuacją.

7. **Wznowienie Ścieżki**: Gdy nadchodzi wiadomość, która pasuje do zablokowanego odbioru lub warunek synchronizacji jest spełniony, odpowiednia ścieżka jest wznawiana i dodawana z powrotem do kolejki planowania.

8. **Łączenie Ścieżek**: Ścieżki potomne mogą być łączone z powrotem do ich ścieżki nadrzędnej, łącząc ich wyniki.

9. **Zakończenie Ścieżki**: Gdy ścieżka kończy swoją sekwencję kodu bajtowego, jej zasoby są zwalniane, a jej kontekst może być zbierany przez odśmiecacz, jeśli nie jest już referencjonowany.

### Alokacja i Planowanie Oparte na Ścieżkach

Alokacja zasobów wykonawczych (czas CPU, pamięć itp.) do wątków jest zarządzana przez system planowania świadomy ścieżek, który mapuje fizyczne zasoby na ścieżki wykonania na podstawie ich kontekstów.

Planista oparty na ścieżkach utrzymuje kilka struktur danych:

1. **Aktywna Kolejka Ścieżek**: Zawiera konteksty ścieżek gotowych do wykonania.
2. **Mapa Zablokowanych Ścieżek**: Mapuje kanały na ścieżki zablokowane na odbiorach.
3. **Mapa Synchronizacji Ścieżek**: Śledzi ścieżki oczekujące na barierach synchronizacji.
4. **Mapa Hierarchii Ścieżek**: Utrzymuje relacje rodzic-dziecko między ścieżkami.
5. **Mapa Zasobów**: Śledzi wykorzystanie zasobów przez każdą ścieżkę.

Gdy wiadomość jest wysyłana na kanale, planista kieruje ją przez odpowiednie ścieżki i sprawdza, czy jakiekolwiek ścieżki są zablokowane na tym kanale. Jeśli tak, przenosi je z mapy zablokowanych do aktywnej kolejki.

Planista wybiera ścieżki z aktywnej kolejki na podstawie polityk planowania (np. round-robin, opartych na priorytetach, świadomych hierarchii ścieżek) i przypisuje je do dostępnych zasobów wykonawczych (np. rdzeni CPU).

To podejście do zarządzania wątkami oparte na ścieżkach oferuje kilka zalet:

- **Planowanie Hierarchiczne**: Planista może wykorzystać hierarchię ścieżek do podejmowania inteligentnych decyzji planowania, priorytetyzując ścieżki na podstawie ich pozycji w hierarchii.
- **Ustrukturyzowana Współbieżność**: Hierarchia ścieżek zapewnia ustrukturyzowany widok współbieżności, ułatwiając rozumowanie o niej i zarządzanie nią.
- **Efektywna Synchronizacja**: Ścieżki mogą synchronizować się na barierach bez złożonych mechanizmów blokowania.
- **Precyzyjna Kontrola Zasobów**: Zasoby mogą być alokowane i kontrolowane na poziomie ścieżki, umożliwiając precyzyjne zarządzanie zasobami.
- **Równoważenie Obciążenia Oparte na Ścieżkach**: W środowisku rozproszonym, całe ścieżki lub poddrzewa ścieżek mogą być migrowane między węzłami fizycznymi w celu równoważenia obciążenia.

### Wykonanie Rozproszone Oparte na Ścieżkach

Model wątków oparty na ścieżkach naturalnie rozszerza się na wykonanie rozproszone na wielu maszynach fizycznych. Ponieważ każda ścieżka jest identyfikowana przez swój kontekst w RSpace, a cała interakcja odbywa się poprzez przekazywanie wiadomości, fizyczna lokalizacja ścieżki jest przezroczysta dla innych ścieżek.

W środowisku rozproszonym:

1. **Routing Kanałów Świadomy Ścieżek**: Wiadomości wysyłane na kanałach są kierowane do odpowiedniego węzła fizycznego na podstawie lokalizacji odbierającej ścieżki.

2. **Migracja Ścieżek**: Całe ścieżki lub poddrzewa ścieżek mogą być migrowane między węzłami w celu równoważenia obciążenia lub tolerancji błędów.

3. **Rozproszone Planowanie Oparte na Ścieżkach**: Rozproszony planista koordynuje alokację zasobów do ścieżek na wielu węzłach, biorąc pod uwagę hierarchię ścieżek.

4. **Tolerancja Błędów na Poziomie Ścieżek**: Jeśli węzeł ulegnie awarii, ścieżki działające na tym węźle mogą być odzyskane z trwałej pamięci i wznowione na innych węzłach, zachowując ich relacje hierarchiczne.

5. **Optymalizacja Lokalności Ścieżek**: Powiązane ścieżki mogą być współlokalizowane na tym samym węźle fizycznym, aby zminimalizować narzut komunikacyjny.

Ten model wykonania rozproszonego oparty na ścieżkach idealnie pasuje do warstwy pamięci RSpace. Reprezentując ścieżki jako jednostki w RSpace, VM może wykorzystać rozproszoną naturę RSpace dla efektywnego wykonania na wielu węzłach. Hierarchia ścieżek zapewnia naturalną strukturę dla dystrybucji obliczeń przy zachowaniu właściwości semantycznych programu.

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
