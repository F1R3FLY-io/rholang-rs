# Next-Generation Rholang Virtual Machine Design

## Introduction & Vision

The next-generation Rholang Virtual Machine (VM) represents a fundamental shift in how concurrent processes are executed in a distributed environment. Unlike traditional virtual machines that manage concurrency through shared memory and locks, this new VM design embraces the true nature of Rholang's concurrency model by treating each process as an independent execution unit with its own isolated state machine.

The core vision is to create a VM architecture where the mathematical properties of the Rho-calculus are preserved throughout the compilation and execution pipeline. This ensures that the semantics of Rholang programs remain consistent from source code to execution, maintaining what we call "functoriality" - the property that compilation preserves the structure of the source language.

By deeply integrating with RSpace (the tuplespace-based storage layer), this VM design enables a natural representation of concurrent processes as independent entities that communicate solely through message passing. This approach not only simplifies the execution model but also provides a clear path to distributed execution across multiple nodes in a network.

## Core Architectural Requirements

### 1. Functoriality of Quoting

The compilation of a quoted process (@P) must be equivalent to a QUOTE operation on the already-compiled bytecode of the process P. This is expressed by the formula:

```
[| @P |] = QUOTE [| P |]
```

This principle ensures that the quoting operation in Rholang maintains its semantic meaning through the compilation process. Quoting is a fundamental operation in the Rho-calculus that converts a process into a name, allowing processes to be treated as first-class values that can be passed in messages.

The technical significance of this requirement is profound: it guarantees that the VM can handle higher-order processes (processes that manipulate other processes) correctly. This is crucial for implementing advanced patterns like mobile code, where processes can be sent between different parts of a distributed system and executed remotely.

### 2. Functoriality of Concurrency

The compilation of a parallel composition of processes (P1 | … | Pn) must result in a multiset ({|...|}) containing the individual compiled bytecodes of each process. This is expressed by the formula:

```
[| P1 | … | Pn |] = {| [| P1 |], …, [| Pn |] |}
```

This principle ensures that the parallel composition operator (|) in Rholang directly translates to independent bytecode sequences in the compiled output. The multiset representation captures the commutative nature of parallel composition - the order of processes doesn't matter, only their concurrent execution.

This requirement is essential for preserving the true concurrency semantics of Rholang. By representing parallel processes as independent bytecode sequences, the VM can execute them truly concurrently without artificial sequentialization, which would introduce non-determinism not present in the source program.

### 3. State as a Multiset

The complete, instantaneous state of the VM is defined as the multiset of individual bytecode sequences:

```
{| [| P1 |], …, [| Pn |] |}
```

This principle establishes that the VM's state is not a monolithic entity but a collection of independent process states. Each process in the system contributes its own state to the overall VM state, without direct interaction with other processes' states.

This approach to state representation is crucial for scalability and fault isolation. Since processes don't share state directly, failures in one process don't corrupt the state of others. Additionally, this model naturally supports distribution, as different processes can be executed on different physical machines without requiring complex state synchronization mechanisms.

### 4. Isolated Execution

Each individual bytecode sequence [| Pi |] from the multiset must be executed in its own separate, dedicated state machine. This state machine represents a single logical thread.

This principle enforces strict isolation between processes, ensuring that each process executes independently with its own execution context, stack, and local variables. This isolation is fundamental to the Rho-calculus model, where processes interact only through explicit communication channels.

The technical significance of isolated execution extends beyond correctness to performance and scalability. By executing processes in isolated state machines, the VM can easily distribute execution across multiple cores or even multiple machines. This approach also simplifies reasoning about process behavior, as each process's execution depends only on its own state and the messages it receives.

### 5. Deep RSpace Integration via the "Channel Trick"

To handle operations that are not native Rholang communications (e.g., arithmetic), each state machine [| Pi |] must be assigned a private channel. This channel is used to communicate with a "primitive processor." This embeds every operation, including primitive ones, into the RSpace communication model.

This principle ensures that all computation in the VM, even primitive operations like arithmetic, is expressed through the same communication mechanism used for process interaction. By assigning each state machine a private channel, the VM can route requests for primitive operations to specialized processors without breaking the message-passing paradigm.

The "Channel Trick" is a powerful unification technique that simplifies the VM architecture by reducing all computation to message passing. This approach makes the VM more extensible, as new primitive operations can be added by simply registering new handlers for specific message patterns, without modifying the core execution engine.

### 6. Thread Abstraction

The "witness" for a thread (be it a green thread or a physical one) is the private channel assigned to a state machine. Thread management becomes a process of mapping execution resources to these channel representatives within RSpace.

This principle establishes a clear identity for each execution thread in the system. By using the private channel as the thread's identity, the VM can track, schedule, and manage threads using the same mechanisms it uses for other RSpace operations.

This approach to thread abstraction provides a natural way to implement features like thread prioritization, load balancing, and resource allocation. It also simplifies the implementation of advanced concurrency patterns like join calculus, where multiple threads synchronize on shared channels.

## Implementation and Execution Model

### The Compilation Pipeline

The compilation pipeline transforms Rholang source code into a multiset of independent bytecode sequences, preserving the concurrency structure of the original program.

1. **Parsing**: The Rholang source code is parsed into an Abstract Syntax Tree (AST) using a parser generated from the Rholang grammar.

2. **AST Analysis**: The AST is analyzed to identify parallel compositions, name declarations, and other Rholang constructs.

3. **Bytecode Generation**: For each process in a parallel composition, the compiler generates a separate bytecode sequence. This is where the functoriality of concurrency is implemented:
   
   ```
   compile(P1 | P2 | ... | Pn) = {| compile(P1), compile(P2), ..., compile(Pn) |}
   ```

4. **Optimization**: Each bytecode sequence is optimized independently, without affecting the semantics of other sequences.

5. **Linking**: References between processes (e.g., through shared channels) are resolved and linked appropriately.

The key innovation in this compilation pipeline is the direct mapping of parallel composition to independent bytecode sequences. This preserves the concurrency structure of the original program and enables truly parallel execution.

### State Machine Execution

Each bytecode sequence [| Pi |] is executed by a dedicated state machine with its own execution context. The state machine maintains:

1. **Instruction Pointer**: Points to the current instruction being executed.
2. **Operand Stack**: Holds intermediate values during computation.
3. **Local Variables**: Stores variables declared within the process.
4. **Channel References**: Maintains references to channels used by the process.
5. **Private Channel**: A unique channel assigned to this state machine for primitive operations.

The execution of a state machine follows a simple cycle:

1. Fetch the next instruction from the bytecode sequence.
2. Decode the instruction to determine the operation.
3. Execute the operation, which may:
   - Modify the operand stack
   - Update local variables
   - Send or receive messages on channels
   - Request primitive operations via the private channel
4. Update the instruction pointer.
5. Repeat until the bytecode sequence is exhausted or blocked on a receive operation.

This isolated execution model simplifies the implementation of the VM and enables straightforward parallelization. Each state machine can be scheduled independently, and multiple state machines can execute concurrently without complex synchronization mechanisms.

### RSpace Interference and the "Channel Trick"

The "Channel Trick" is a key innovation that embeds all computation, including primitive operations, into the RSpace communication model. Here's a detailed walkthrough of how a state machine executes a primitive operation, such as `x = 5 + 3`:

1. **Instruction Decoding**: The state machine encounters an ADD instruction in its bytecode sequence.

2. **Operand Preparation**: The operands (5 and 3) are already on the operand stack, having been pushed there by previous instructions.

3. **Private Channel Communication**:
   - The state machine creates a message containing:
     - The operation code (ADD)
     - The operands (5 and 3)
     - A continuation channel for the result
   - This message is sent on the state machine's private channel to the primitive processor.

4. **Primitive Processing**:
   - The primitive processor receives the message from the private channel.
   - It performs the requested operation (5 + 3 = 8).
   - It sends the result (8) on the continuation channel.

5. **Result Reception**:
   - The state machine receives the result (8) from the continuation channel.
   - It pushes the result onto the operand stack.
   - It continues execution with the next instruction.

This approach has several advantages:

- **Uniformity**: All operations, whether primitive or high-level, use the same communication mechanism.
- **Extensibility**: New primitive operations can be added by registering new handlers with the primitive processor.
- **Distribution**: Primitive operations can be executed on different physical machines from the requesting state machine.
- **Fault Isolation**: Failures in primitive operations don't affect the state of other processes.

Most importantly, this approach makes the collection of state machines natively representable within RSpace. Each state machine is identified by its private channel, and all interaction with the state machine happens through message passing on that channel. This enables a clean integration with the RSpace storage layer and provides a natural path to distributed execution.

## Thread Invocation and Management

The question "Where are the threads invoked?" has a clear answer in this architecture: threads are invoked at the level of individual state machines, each identified by its private channel in RSpace.

### Thread Identity and Lifecycle

In this architecture, a thread is not a traditional OS thread or even a green thread in the conventional sense. Instead, a thread is an execution context associated with a specific state machine, identified by its private channel in RSpace.

The lifecycle of a thread follows these stages:

1. **Creation**: When a new process is spawned (either from the initial program or through a parallel composition), a new state machine is created with its own private channel.

2. **Scheduling**: The scheduler selects state machines to execute based on resource availability and scheduling policies. The private channel serves as the handle for scheduling decisions.

3. **Execution**: The selected state machine executes its bytecode sequence until it completes or blocks on a receive operation.

4. **Blocking**: If a state machine blocks on a receive operation, its state is preserved, and it's removed from the active scheduling queue until a matching message arrives.

5. **Resumption**: When a message arrives that matches a blocked receive, the corresponding state machine is resumed and added back to the scheduling queue.

6. **Termination**: When a state machine completes its bytecode sequence, its resources are released, and its private channel may be garbage collected if no longer referenced.

### Thread Allocation and Scheduling

The allocation of execution resources (CPU time, memory, etc.) to threads is managed through a scheduling system that maps physical resources to state machines based on their private channels.

The scheduler maintains several data structures:

1. **Active Queue**: Contains private channels of state machines ready for execution.
2. **Blocked Map**: Maps channels to state machines blocked on receives.
3. **Resource Map**: Tracks resource usage by each state machine.

When a message is sent on a channel, the scheduler checks if any state machines are blocked on that channel. If so, it moves them from the blocked map to the active queue.

The scheduler then selects state machines from the active queue based on scheduling policies (e.g., round-robin, priority-based) and assigns them to available execution resources (e.g., CPU cores).

This approach to thread management has several advantages:

- **Scalability**: The number of state machines can far exceed the number of physical CPU cores, allowing for efficient utilization of resources.
- **Fairness**: The scheduler can implement various fairness policies to ensure that all processes get a chance to execute.
- **Resource Control**: The scheduler can limit the resources allocated to each state machine, preventing resource exhaustion.
- **Load Balancing**: In a distributed setting, state machines can be migrated between physical nodes to balance load.

### Distributed Execution

The thread model described above naturally extends to distributed execution across multiple physical machines. Since each state machine is identified by its private channel, and all interaction happens through message passing, the physical location of a state machine is transparent to other processes.

In a distributed setting:

1. **Channel Routing**: Messages sent on channels are routed to the appropriate physical node based on the location of the receiving state machine.

2. **State Machine Migration**: State machines can be migrated between nodes for load balancing or fault tolerance.

3. **Distributed Scheduling**: A distributed scheduler coordinates the allocation of resources across multiple nodes.

4. **Fault Tolerance**: If a node fails, the state machines running on that node can be recovered from persistent storage and resumed on other nodes.

This distributed execution model aligns perfectly with the RSpace storage layer, which already provides mechanisms for distributed storage and retrieval of tuples. By representing state machines as entities in RSpace, the VM can leverage these mechanisms for distributed execution.

## Conclusion

The next-generation Rholang VM design presented in this document represents a significant advancement in the execution of concurrent, distributed programs. By adhering to the six core principles - functoriality of quoting, functoriality of concurrency, state as a multiset, isolated execution, deep RSpace integration, and thread abstraction - this design creates a VM that truly embodies the mathematical foundations of the Rho-calculus.

The key innovations in this design include:

1. The direct representation of parallel composition as independent bytecode sequences.
2. The execution of each process in its own isolated state machine.
3. The use of private channels to embed all computation into the RSpace communication model.
4. The identification of threads with private channels in RSpace.

These innovations enable a VM that is not only mathematically consistent but also highly scalable, fault-tolerant, and naturally distributed. By building on the solid foundation of the Rho-calculus and integrating deeply with RSpace, this VM design provides a powerful platform for the next generation of concurrent, distributed applications.

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

Kluczowe innowacje w tym projekcie obejmują:

1. Bezpośrednią reprezentację kompozycji równoległej jako niezależnych sekwencji kodu bajtowego.
2. Wykonanie każdego procesu w jego własnej izolowanej maszynie stanów.
3. Wykorzystanie prywatnych kanałów do osadzenia wszystkich obliczeń w modelu komunikacji RSpace.
4. Identyfikację wątków z prywatnymi kanałami w RSpace.

Te innowacje umożliwiają VM, która jest nie tylko matematycznie spójna, ale także wysoce skalowalna, odporna na błędy i naturalnie rozproszona. Budując na solidnych podstawach rachunku Rho i integrując się głęboko z RSpace, ten projekt VM zapewnia potężną platformę dla następnej generacji współbieżnych, rozproszonych aplikacji.